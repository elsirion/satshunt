use crate::models::{AuthMethod, User};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use tower_sessions::Session;

const SESSION_USER_KEY: &str = "user_id";

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

/// Session-based authentication extractor
pub struct AuthUser {
    pub user_id: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                tracing::error!("Failed to extract session");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            })?;

        let user_id: Option<String> = session
            .get(SESSION_USER_KEY)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get user from session: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            })?;

        match user_id {
            Some(user_id) => Ok(AuthUser { user_id }),
            None => {
                tracing::debug!("User not authenticated, redirecting to login");
                Err(Redirect::to("/login").into_response())
            }
        }
    }
}

/// Optional authentication - doesn't redirect if not authenticated
/// Used for pages that show different content for authenticated vs unauthenticated users
pub struct OptionalAuthUser {
    pub user_id: Option<String>,
}

#[async_trait]
impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                tracing::error!("Failed to extract session");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            })?;

        let user_id: Option<String> = session
            .get(SESSION_USER_KEY)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get user from session: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            })?;

        Ok(OptionalAuthUser { user_id })
    }
}

/// Helper to store user ID in session
pub async fn login_user(session: &Session, user_id: &str) -> anyhow::Result<()> {
    session
        .insert(SESSION_USER_KEY, user_id.to_string())
        .await?;
    Ok(())
}

/// Helper to remove user from session
pub async fn logout_user(session: &Session) -> anyhow::Result<()> {
    session.remove::<String>(SESSION_USER_KEY).await?;
    Ok(())
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
        AuthMethod::Password { password_hash } => {
            verify_password(password, &password_hash)
        }
        _ => Err(anyhow::anyhow!("User does not use password authentication")),
    }
}
