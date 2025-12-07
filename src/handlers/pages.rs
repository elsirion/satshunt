use crate::{
    auth::{hash_password, login_user, logout_user, verify_user_password, AuthUser, LoginRequest, RegisterRequest, OptionalAuthUser},
    handlers::api::AppState,
    models::AuthMethod,
    templates,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect, Response, IntoResponse},
    Form,
};
use serde::Deserialize;
use std::sync::Arc;
use tower_sessions::Session;

#[derive(Deserialize)]
pub struct ErrorQuery {
    error: Option<String>,
}

pub async fn home_page(
    State(state): State<Arc<AppState>>,
    opt_auth: OptionalAuthUser,
) -> Result<Html<String>, StatusCode> {
    let stats = state.db.get_stats().await.map_err(|e| {
        tracing::error!("Failed to get stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let username = match opt_auth.user_id {
        Some(user_id) => state
            .db
            .get_user_by_id(&user_id)
            .await
            .ok()
            .flatten()
            .map(|user| user.username),
        None => None,
    };

    let content = templates::home(&stats);
    let page = templates::base_with_user("Home", content, username.as_deref());

    Ok(Html(page.into_string()))
}

pub async fn map_page(
    State(state): State<Arc<AppState>>,
    opt_auth: OptionalAuthUser,
) -> Result<Html<String>, StatusCode> {
    let locations = state.db.list_active_locations().await.map_err(|e| {
        tracing::error!("Failed to get active locations: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let username = match opt_auth.user_id {
        Some(user_id) => state
            .db
            .get_user_by_id(&user_id)
            .await
            .ok()
            .flatten()
            .map(|user| user.username),
        None => None,
    };

    let content = templates::map(&locations, state.max_sats_per_location);
    let page = templates::base_with_user("Map", content, username.as_deref());

    Ok(Html(page.into_string()))
}

pub async fn new_location_page(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Html<String>, StatusCode> {
    let username = state
        .db
        .get_user_by_id(&auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get user: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(|user| user.username);

    let content = templates::new_location();
    let page = templates::base_with_user("Add Location", content, username.as_deref());

    Ok(Html(page.into_string()))
}

pub async fn location_detail_page(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    opt_auth: OptionalAuthUser,
) -> Result<Html<String>, StatusCode> {
    let location = state.db
        .get_location(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let photos = state.db.get_photos_for_location(&id).await.map_err(|e| {
        tracing::error!("Failed to get photos: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let username = match opt_auth.user_id {
        Some(user_id) => state
            .db
            .get_user_by_id(&user_id)
            .await
            .ok()
            .flatten()
            .map(|user| user.username),
        None => None,
    };

    let content = templates::location_detail(&location, &photos, &state.base_url, state.max_sats_per_location);
    let page = templates::base_with_user(&location.name, content, username.as_deref());

    Ok(Html(page.into_string()))
}

pub async fn nfc_setup_page(
    State(state): State<Arc<AppState>>,
    Path(write_token): Path<String>,
    opt_auth: OptionalAuthUser,
) -> Result<Html<String>, StatusCode> {
    let location = state.db
        .get_location_by_write_token(&write_token)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get location by write token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let username = match opt_auth.user_id {
        Some(user_id) => state
            .db
            .get_user_by_id(&user_id)
            .await
            .ok()
            .flatten()
            .map(|user| user.username),
        None => None,
    };

    let content = templates::nfc_setup(&location, &write_token, &state.base_url);
    let page = templates::base_with_user("NFC Setup", content, username.as_deref());

    Ok(Html(page.into_string()))
}

pub async fn donate_page(
    State(state): State<Arc<AppState>>,
    opt_auth: OptionalAuthUser,
) -> Result<Html<String>, StatusCode> {
    let pool = state.db.get_donation_pool().await.map_err(|e| {
        tracing::error!("Failed to get donation pool: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let username = match opt_auth.user_id {
        Some(user_id) => state
            .db
            .get_user_by_id(&user_id)
            .await
            .ok()
            .flatten()
            .map(|user| user.username),
        None => None,
    };

    let content = templates::donate(&pool);
    let page = templates::base_with_user("Donate", content, username.as_deref());

    Ok(Html(page.into_string()))
}

pub async fn login_page(Query(params): Query<ErrorQuery>) -> Html<String> {
    let content = templates::login(params.error.as_deref());
    let page = templates::base("Login", content);
    Html(page.into_string())
}

pub async fn register_page(Query(params): Query<ErrorQuery>) -> Html<String> {
    let content = templates::register(params.error.as_deref());
    let page = templates::base("Register", content);
    Html(page.into_string())
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    session: Session,
    Form(login_req): Form<LoginRequest>,
) -> Response {
    // Get user by username
    let user = match state.db.get_user_by_username(&login_req.username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            tracing::warn!("Login attempt for non-existent user: {}", login_req.username);
            return Redirect::to("/login?error=Invalid%20username%20or%20password").into_response();
        }
        Err(e) => {
            tracing::error!("Database error during login: {}", e);
            return Redirect::to("/login?error=An%20error%20occurred.%20Please%20try%20again.").into_response();
        }
    };

    // Verify password
    match verify_user_password(&user, &login_req.password) {
        Ok(true) => {
            // Password is correct, create session
            if let Err(e) = login_user(&session, &user.id).await {
                tracing::error!("Failed to create session: {}", e);
                return Redirect::to("/login?error=An%20error%20occurred.%20Please%20try%20again.").into_response();
            }

            // Update last login time
            if let Err(e) = state.db.update_last_login(&user.id).await {
                tracing::error!("Failed to update last login: {}", e);
                // Don't fail the login for this
            }

            tracing::info!("User {} logged in successfully", user.username);
            Redirect::to("/").into_response()
        }
        Ok(false) => {
            tracing::warn!("Failed login attempt for user: {}", login_req.username);
            Redirect::to("/login?error=Invalid%20username%20or%20password").into_response()
        }
        Err(e) => {
            tracing::error!("Error verifying password: {}", e);
            Redirect::to("/login?error=An%20error%20occurred.%20Please%20try%20again.").into_response()
        }
    }
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    session: Session,
    Form(register_req): Form<RegisterRequest>,
) -> Response {
    // Validate username is not empty
    if register_req.username.trim().is_empty() {
        return Redirect::to("/register?error=Username%20cannot%20be%20empty").into_response();
    }

    // Validate password is not empty
    if register_req.password.is_empty() {
        return Redirect::to("/register?error=Password%20cannot%20be%20empty").into_response();
    }

    // Check if username already exists
    match state.db.get_user_by_username(&register_req.username).await {
        Ok(Some(_)) => {
            tracing::warn!("Registration attempt with existing username: {}", register_req.username);
            return Redirect::to("/register?error=Username%20already%20exists").into_response();
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("Database error checking username: {}", e);
            return Redirect::to("/register?error=An%20error%20occurred.%20Please%20try%20again.").into_response();
        }
    }

    // Hash password
    let password_hash = match hash_password(&register_req.password) {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!("Failed to hash password: {}", e);
            return Redirect::to("/register?error=An%20error%20occurred.%20Please%20try%20again.").into_response();
        }
    };

    // Create user with password auth method
    let auth_method = AuthMethod::Password { password_hash };
    let user = match state.db.create_user(
        register_req.username.clone(),
        register_req.email.filter(|e| !e.is_empty()),
        auth_method,
    ).await {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("Failed to create user: {}", e);
            return Redirect::to("/register?error=An%20error%20occurred.%20Please%20try%20again.").into_response();
        }
    };

    // Log the user in immediately
    if let Err(e) = login_user(&session, &user.id).await {
        tracing::error!("Failed to create session after registration: {}", e);
        return Redirect::to("/register?error=An%20error%20occurred.%20Please%20try%20again.").into_response();
    }

    tracing::info!("New user registered: {}", user.username);
    Redirect::to("/").into_response()
}

pub async fn logout(session: Session) -> Response {
    if let Err(e) = logout_user(&session).await {
        tracing::error!("Failed to logout: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Redirect::to("/").into_response()
}

pub async fn profile_page(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Html<String>, StatusCode> {
    // Get user data
    let user = state
        .db
        .get_user_by_id(&auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get user: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::error!("User not found: {}", auth.user_id);
            StatusCode::NOT_FOUND
        })?;

    // Get user's locations
    let locations = state
        .db
        .get_locations_by_user(&auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get user locations: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let content = templates::profile(&user, &locations, state.max_sats_per_location);
    let page = templates::base_with_user("Profile", content, Some(&user.username));

    Ok(Html(page.into_string()))
}
