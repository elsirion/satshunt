use crate::{
    auth::{
        hash_password, remove_user_cookie, set_user_cookie, verify_user_password, CookieUser,
        LoginRequest, RegisterRequest, RequireRegistered, UserKind,
    },
    handlers::api::AppState,
    models::AuthMethod,
    ntag424, templates,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ErrorQuery {
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct LocationDetailQuery {
    pub error: Option<String>,
    pub success: Option<String>,
    pub amount: Option<i64>,
}

#[derive(Deserialize)]
pub struct WithdrawQuery {
    /// NTAG424 encrypted picc_data (parameter name: p)
    #[serde(alias = "picc_data")]
    pub p: Option<String>,
    /// NTAG424 CMAC signature (parameter name: c)
    #[serde(alias = "cmac")]
    pub c: Option<String>,
    pub error: Option<String>,
}

/// Helper to get username for navbar from UserKind
fn get_navbar_username(kind: &UserKind) -> Option<String> {
    match kind {
        UserKind::Registered { username } => Some(username.clone()),
        _ => None,
    }
}

pub async fn home_page(State(state): State<Arc<AppState>>, user: CookieUser) -> impl IntoResponse {
    let stats = match state.db.get_stats().await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to get stats: {}", e);
            return (user.jar, StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let username = get_navbar_username(&user.kind);
    let content = templates::home(&stats);
    let page = templates::base_with_user("Home", content, username.as_deref());

    (user.jar, Html(page.into_string())).into_response()
}

pub async fn map_page(State(state): State<Arc<AppState>>, user: CookieUser) -> impl IntoResponse {
    let locations = match state.db.list_active_locations().await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to get active locations: {}", e);
            return (user.jar, StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let username = get_navbar_username(&user.kind);
    let content = templates::map(&locations, state.max_sats_per_location);
    let page = templates::base_with_user("Map", content, username.as_deref());

    (user.jar, Html(page.into_string())).into_response()
}

pub async fn new_location_page(auth: RequireRegistered) -> impl IntoResponse {
    let content = templates::new_location();
    let page = templates::base_with_user("Add Location", content, Some(&auth.username));
    (auth.jar, Html(page.into_string()))
}

pub async fn location_detail_page(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<LocationDetailQuery>,
    user: CookieUser,
) -> impl IntoResponse {
    let location = match state.db.get_location(&id).await {
        Ok(Some(l)) => l,
        Ok(None) => return (user.jar, StatusCode::NOT_FOUND).into_response(),
        Err(e) => {
            tracing::error!("Failed to get location: {}", e);
            return (user.jar, StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let photos = match state.db.get_photos_for_location(&id).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to get photos: {}", e);
            return (user.jar, StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let scans = match state.db.get_scans_for_location(&id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to get scans: {}", e);
            return (user.jar, StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let refills = match state.db.get_refills_for_location(&id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to get refills: {}", e);
            return (user.jar, StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let current_user_id = Some(user.user_id.as_str());
    let username = get_navbar_username(&user.kind);

    let content = templates::location_detail(
        &location,
        &photos,
        &scans,
        &refills,
        state.max_sats_per_location,
        current_user_id,
        params.error.as_deref(),
        params.success.as_deref(),
        params.amount,
        &state.base_url,
    );
    let page = templates::base_with_user(&location.name, content, username.as_deref());

    (user.jar, Html(page.into_string())).into_response()
}

pub async fn nfc_setup_page(
    State(state): State<Arc<AppState>>,
    Path(write_token): Path<String>,
    user: CookieUser,
) -> impl IntoResponse {
    // Get location by write token and redirect to location detail page
    let location = match state.db.get_location_by_write_token(&write_token).await {
        Ok(Some(l)) => l,
        Ok(None) => return (user.jar, StatusCode::NOT_FOUND).into_response(),
        Err(e) => {
            tracing::error!("Failed to get location by write token: {}", e);
            return (user.jar, StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    // Redirect to location detail page where NFC setup is now integrated
    (
        user.jar,
        Redirect::to(&format!("/locations/{}", location.id)),
    )
        .into_response()
}

pub async fn donate_page(
    State(state): State<Arc<AppState>>,
    user: CookieUser,
) -> impl IntoResponse {
    let pool = match state.db.get_donation_pool().await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to get donation pool: {}", e);
            return (user.jar, StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let completed_donations = match state.db.list_completed_donations().await {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Failed to get completed donations: {}", e);
            return (user.jar, StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let username = get_navbar_username(&user.kind);
    let content = templates::donate(&pool, &completed_donations);
    let page = templates::base_with_user("Donate", content, username.as_deref());

    (user.jar, Html(page.into_string())).into_response()
}

pub async fn login_page(Query(params): Query<ErrorQuery>, user: CookieUser) -> impl IntoResponse {
    let content = templates::login(params.error.as_deref());
    let page = templates::base("Login", content);
    (user.jar, Html(page.into_string()))
}

pub async fn register_page(
    Query(params): Query<ErrorQuery>,
    user: CookieUser,
) -> impl IntoResponse {
    let content = templates::register(params.error.as_deref());
    let page = templates::base("Register", content);
    (user.jar, Html(page.into_string()))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    user: CookieUser,
    Form(login_req): Form<LoginRequest>,
) -> impl IntoResponse {
    // Get user by username
    let db_user = match state.db.get_user_by_username(&login_req.username).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            tracing::warn!(
                "Login attempt for non-existent user: {}",
                login_req.username
            );
            return (
                user.jar,
                Redirect::to("/login?error=Invalid%20username%20or%20password"),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Database error during login: {}", e);
            return (
                user.jar,
                Redirect::to("/login?error=An%20error%20occurred.%20Please%20try%20again."),
            )
                .into_response();
        }
    };

    // Verify password
    match verify_user_password(&db_user, &login_req.password) {
        Ok(true) => {
            // Password is correct, set cookie to point to this user
            let jar = set_user_cookie(user.jar, &db_user.id);

            // Update last login time
            if let Err(e) = state.db.update_last_login(&db_user.id).await {
                tracing::error!("Failed to update last login: {}", e);
                // Don't fail the login for this
            }

            tracing::info!("User {} logged in successfully", db_user.display_name());
            (jar, Redirect::to("/")).into_response()
        }
        Ok(false) => {
            tracing::warn!("Failed login attempt for user: {}", login_req.username);
            (
                user.jar,
                Redirect::to("/login?error=Invalid%20username%20or%20password"),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Error verifying password: {}", e);
            (
                user.jar,
                Redirect::to("/login?error=An%20error%20occurred.%20Please%20try%20again."),
            )
                .into_response()
        }
    }
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    user: CookieUser,
    Form(register_req): Form<RegisterRequest>,
) -> impl IntoResponse {
    // Validate username is not empty
    if register_req.username.trim().is_empty() {
        return (
            user.jar,
            Redirect::to("/register?error=Username%20cannot%20be%20empty"),
        )
            .into_response();
    }

    // Validate password is not empty
    if register_req.password.is_empty() {
        return (
            user.jar,
            Redirect::to("/register?error=Password%20cannot%20be%20empty"),
        )
            .into_response();
    }

    // Check if username already exists
    match state.db.get_user_by_username(&register_req.username).await {
        Ok(Some(_)) => {
            tracing::warn!(
                "Registration attempt with existing username: {}",
                register_req.username
            );
            return (
                user.jar,
                Redirect::to("/register?error=Username%20already%20exists"),
            )
                .into_response();
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("Database error checking username: {}", e);
            return (
                user.jar,
                Redirect::to("/register?error=An%20error%20occurred.%20Please%20try%20again."),
            )
                .into_response();
        }
    }

    // Hash password
    let password_hash = match hash_password(&register_req.password) {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!("Failed to hash password: {}", e);
            return (
                user.jar,
                Redirect::to("/register?error=An%20error%20occurred.%20Please%20try%20again."),
            )
                .into_response();
        }
    };

    // Create user with password auth method
    let auth_method = AuthMethod::Password { password_hash };
    let db_user = match state
        .db
        .create_user(
            register_req.username.clone(),
            register_req.email.filter(|e| !e.is_empty()),
            auth_method,
        )
        .await
    {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("Failed to create user: {}", e);
            return (
                user.jar,
                Redirect::to("/register?error=An%20error%20occurred.%20Please%20try%20again."),
            )
                .into_response();
        }
    };

    // Set cookie to point to the new user
    let jar = set_user_cookie(user.jar, &db_user.id);

    tracing::info!("New user registered: {}", db_user.display_name());
    (jar, Redirect::to("/")).into_response()
}

pub async fn logout(user: CookieUser) -> impl IntoResponse {
    // Remove the user cookie (generates a new anonymous ID)
    let jar = remove_user_cookie(user.jar);
    (jar, Redirect::to("/"))
}

pub async fn profile_page(
    State(state): State<Arc<AppState>>,
    auth: RequireRegistered,
) -> impl IntoResponse {
    // Get user data
    let user = match state.db.get_user_by_id(&auth.user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            tracing::error!("User not found: {}", auth.user_id);
            return (auth.jar, StatusCode::NOT_FOUND).into_response();
        }
        Err(e) => {
            tracing::error!("Failed to get user: {}", e);
            return (auth.jar, StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    // Get user's locations
    let locations = match state.db.get_locations_by_user(&auth.user_id).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to get user locations: {}", e);
            return (auth.jar, StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let content = templates::profile(&user, &locations, state.max_sats_per_location);
    let display_name = user.display_name();
    let page = templates::base_with_user("Profile", content, Some(&display_name));

    (auth.jar, Html(page.into_string())).into_response()
}

/// Withdraw/Collection page - displays the collection UI for the custodial wallet.
///
/// The URL should contain picc_data and cmac query parameters from the NFC chip
/// for counter verification.
///
/// On first scan of a programmed location, this activates the location and
/// redirects to the location details page.
pub async fn withdraw_page(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
    Query(params): Query<WithdrawQuery>,
    user: CookieUser,
) -> impl IntoResponse {
    // Get the location
    let location = match state.db.get_location(&location_id).await {
        Ok(Some(l)) => l,
        Ok(None) => return (user.jar, StatusCode::NOT_FOUND).into_response(),
        Err(e) => {
            tracing::error!("Failed to get location: {}", e);
            return (user.jar, StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    // Get user's current balance (if they have any transactions)
    let user_balance_msats = state.db.get_user_balance(&user.user_id).await.unwrap_or(0);
    let user_balance_sats = user_balance_msats / 1000;

    // Get user info from DB for template
    let db_user = state.db.get_user_by_id(&user.user_id).await.ok().flatten();

    let is_new_user = matches!(user.kind, UserKind::AnonNew);

    // Check if we have SUN parameters
    let (picc_data, cmac) = match (&params.p, &params.c) {
        (Some(p), Some(c)) => (p.clone(), c.clone()),
        _ => {
            // No SUN parameters - show error
            let content = templates::collect(templates::CollectParams {
                location: &location,
                available_sats: location.current_sats(),
                current_balance_sats: user_balance_sats,
                picc_data: "",
                cmac: "",
                error: Some("Invalid NFC scan. Please scan the sticker again."),
                is_new_user,
                user: db_user.as_ref(),
            });
            let page = templates::base("Collect Sats", content);
            return (user.jar, Html(page.into_string())).into_response();
        }
    };

    // Verify the SUN message
    let verification =
        ntag424::verify_sun_message(&state.db, &location_id, &picc_data, &cmac).await;

    // Handle first scan activation - if location is programmed but not active,
    // activate it and redirect to location details page
    if location.is_programmed() {
        match &verification {
            Ok(v) => {
                // Update counter to prevent replay
                if let Err(e) = state
                    .db
                    .update_nfc_card_counter(&v.nfc_card.location_id, v.counter as i64)
                    .await
                {
                    tracing::error!("Failed to update NFC card counter: {}", e);
                    return (user.jar, StatusCode::INTERNAL_SERVER_ERROR).into_response();
                }

                // Activate the location
                if let Err(e) = state
                    .db
                    .update_location_status(&location.id, "active")
                    .await
                {
                    tracing::error!("Failed to activate location: {}", e);
                    return (user.jar, StatusCode::INTERNAL_SERVER_ERROR).into_response();
                }

                tracing::info!("Location {} activated on first scan", location.name);

                // Redirect to location details page
                return (
                    user.jar,
                    Redirect::to(&format!("/locations/{}?success=activated", location.id)),
                )
                    .into_response();
            }
            Err(e) => {
                tracing::error!("First scan verification failed: {}", e);
                // Fall through to show error on collection page
            }
        }
    }

    // For active locations, show the collection page
    let error_message = match verification {
        Ok(_) => None,
        Err(ntag424::SunError::ReplayDetected { .. }) => {
            Some("This scan has already been used. Please scan the sticker again.")
        }
        Err(ntag424::SunError::CmacMismatch) => {
            Some("Invalid NFC scan. Please scan the sticker again.")
        }
        Err(ntag424::SunError::UidMismatch { .. }) => {
            Some("Invalid NFC card. This card is not associated with this location.")
        }
        Err(ntag424::SunError::CardNotFound) | Err(ntag424::SunError::CardNotProgrammed) => {
            Some("NFC card not configured. Please contact the location owner.")
        }
        Err(e) => {
            tracing::error!("SUN verification error: {}", e);
            Some("Verification failed. Please try scanning again.")
        }
    };

    // Combine with any error from query params
    let error = params.error.as_deref().or(error_message);

    let content = templates::collect(templates::CollectParams {
        location: &location,
        available_sats: location.current_sats(),
        current_balance_sats: user_balance_sats,
        picc_data: &picc_data,
        cmac: &cmac,
        error,
        is_new_user,
        user: db_user.as_ref(),
    });
    let page = templates::base("Collect Sats", content);

    (user.jar, Html(page.into_string())).into_response()
}

/// Query parameters for wallet page
#[derive(Deserialize)]
pub struct WalletQuery {
    pub success: Option<String>,
    pub amount: Option<i64>,
    pub location: Option<String>,
}

/// Wallet page - shows user's balance and transaction history.
pub async fn wallet_page(
    State(state): State<Arc<AppState>>,
    Query(params): Query<WalletQuery>,
    user: CookieUser,
) -> impl IntoResponse {
    tracing::debug!(
        "Wallet page: user_id={}, kind={:?}",
        user.user_id,
        user.kind
    );

    // Get user balance
    let balance_msats = state.db.get_user_balance(&user.user_id).await.unwrap_or(0);
    let balance_sats = balance_msats / 1000;

    // Get recent transactions
    let transactions = state
        .db
        .get_user_transactions(&user.user_id, 50)
        .await
        .unwrap_or_default();

    // Get user from DB for template
    let db_user = state.db.get_user_by_id(&user.user_id).await.ok().flatten();

    tracing::debug!(
        "Wallet page: user from DB = {:?}",
        db_user
            .as_ref()
            .map(|u| (&u.id, &u.username, &u.auth_method))
    );

    // Build content
    let content = templates::wallet(
        balance_sats,
        &transactions,
        db_user.as_ref(),
        params.success.as_deref(),
        params.amount,
        params.location.as_deref(),
    );
    let username = get_navbar_username(&user.kind);
    let page = templates::base_with_user("My Wallet", content, username.as_deref());

    (user.jar, Html(page.into_string())).into_response()
}
