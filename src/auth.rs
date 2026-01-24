use crate::handlers::api::AppState;
use crate::models::{AuthMethod, User};
use ::time::Duration;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Redirect, Response},
};
pub use axum_extra::extract::cookie::Key;
use axum_extra::extract::cookie::{Cookie, PrivateCookieJar};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

/// Cookie name for user identification
pub const USER_COOKIE_NAME: &str = "satshunt_uid";

/// Cookie max age: 5 years
const COOKIE_MAX_AGE_DAYS: i64 = 365 * 5;

/// The kind of user making a request
#[derive(Debug, Clone)]
pub enum UserKind {
    /// Anonymous user with no database entry yet (brand new visitor)
    AnonNew,
    /// Anonymous user with a database entry (has collected sats before)
    AnonExisting,
    /// Registered user with username
    Registered { username: String },
}

impl UserKind {
    /// Check if this is an anonymous user (new or existing)
    pub fn is_anonymous(&self) -> bool {
        matches!(self, UserKind::AnonNew | UserKind::AnonExisting)
    }

    /// Check if this is a registered user
    pub fn is_registered(&self) -> bool {
        matches!(self, UserKind::Registered { .. })
    }

    /// Get the display name for this user
    pub fn display_name(&self, user_id: &str) -> String {
        match self {
            UserKind::Registered { username } => username.clone(),
            _ => format!("anon_{}", &user_id[..8.min(user_id.len())]),
        }
    }
}

/// Unified user extractor using PrivateCookieJar.
///
/// Every request gets a user ID (from cookie or newly generated).
/// The `kind` field indicates what type of user this is based on DB lookup.
/// The `updated_jar` must be included in the response to persist new/updated cookies.
pub struct CookieUser {
    /// The user's UUID (always present)
    pub user_id: String,
    /// The kind of user (anon new, anon existing, or registered)
    pub kind: UserKind,
    /// Whether a new cookie was created (should set in response)
    pub is_new_cookie: bool,
    /// The updated cookie jar - MUST be included in the response
    pub jar: PrivateCookieJar,
}

impl CookieUser {
    /// Get the display name for this user
    pub fn display_name(&self) -> String {
        self.kind.display_name(&self.user_id)
    }

    /// Check if this is an anonymous user
    pub fn is_anonymous(&self) -> bool {
        self.kind.is_anonymous()
    }

    /// Check if this is a registered user
    pub fn is_registered(&self) -> bool {
        self.kind.is_registered()
    }

    /// Build a cookie for the given user ID
    fn build_cookie(user_id: &str) -> Cookie<'static> {
        Cookie::build((USER_COOKIE_NAME, user_id.to_string()))
            .path("/")
            .http_only(true)
            .secure(false) // Set to true in production with HTTPS
            .max_age(Duration::days(COOKIE_MAX_AGE_DAYS))
            .build()
    }
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for CookieUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        // Extract the private cookie jar using the key from state
        let jar = PrivateCookieJar::from_headers(&parts.headers, state.cookie_key.clone());

        // Check for existing user cookie
        let (user_id, is_new_cookie, jar) = match jar.get(USER_COOKIE_NAME) {
            Some(cookie) => {
                let user_id = cookie.value().to_string();
                tracing::debug!("CookieUser: Found existing user {}", user_id);
                (user_id, false, jar)
            }
            None => {
                // Generate new UUID and add cookie
                let user_id = Uuid::new_v4().to_string();
                tracing::debug!("CookieUser: Generated new user {}", user_id);
                let cookie = CookieUser::build_cookie(&user_id);
                let jar = jar.add(cookie);
                (user_id, true, jar)
            }
        };

        // Look up user in database to determine kind
        let kind = match state.db.get_user_by_id(&user_id).await {
            Ok(Some(user)) => {
                if user.is_anonymous() {
                    UserKind::AnonExisting
                } else {
                    UserKind::Registered {
                        username: user.display_name(),
                    }
                }
            }
            Ok(None) => UserKind::AnonNew,
            Err(e) => {
                tracing::error!("Failed to look up user {}: {}", user_id, e);
                UserKind::AnonNew
            }
        };

        tracing::debug!(
            "CookieUser: user_id={}, kind={:?}, is_new_cookie={}",
            user_id,
            kind,
            is_new_cookie
        );

        Ok(CookieUser {
            user_id,
            kind,
            is_new_cookie,
            jar,
        })
    }
}

/// Extractor that requires a registered user, redirecting to login if not authenticated.
pub struct RequireRegistered {
    pub user_id: String,
    pub username: String,
    pub jar: PrivateCookieJar,
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for RequireRegistered {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let cookie_user = CookieUser::from_request_parts(parts, state).await?;

        match cookie_user.kind {
            UserKind::Registered { username } => Ok(RequireRegistered {
                user_id: cookie_user.user_id,
                username,
                jar: cookie_user.jar,
            }),
            _ => {
                tracing::debug!("User not registered, redirecting to login");
                Err(Redirect::to("/login").into_response())
            }
        }
    }
}

// Keep old type aliases for backwards compatibility during migration
pub type AuthUser = RequireRegistered;

/// Hash a password using Argon2
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
    Ok(password_hash.to_string())
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| anyhow::anyhow!("Failed to parse password hash: {}", e))?;
    let argon2 = Argon2::default();

    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

/// Verify user credentials for password-based authentication
pub fn verify_user_password(user: &User, password: &str) -> anyhow::Result<bool> {
    let auth_method = user.get_auth_method()?;

    match auth_method {
        AuthMethod::Password { password_hash } => verify_password(password, &password_hash),
        _ => Err(anyhow::anyhow!("User does not use password authentication")),
    }
}

/// Set the user cookie to point to a specific user ID (used after login/register).
/// Returns the updated jar that must be included in the response.
pub fn set_user_cookie(jar: PrivateCookieJar, user_id: &str) -> PrivateCookieJar {
    let cookie = CookieUser::build_cookie(user_id);
    jar.add(cookie)
}

/// Remove the user cookie (used for logout).
/// Returns the updated jar that must be included in the response.
pub fn remove_user_cookie(jar: PrivateCookieJar) -> PrivateCookieJar {
    jar.remove(Cookie::from(USER_COOKIE_NAME))
}
