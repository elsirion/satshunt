use crate::{
    auth::{
        hash_password, remove_user_cookie, set_user_cookie, verify_user_password, CookieUser,
        LoginRequest, RegisterRequest, UserKind,
    },
    balance::compute_balance_msats,
    handlers::api::{create_withdraw_token, AppState},
    models::{AuthMethod, UserRole},
    ntag424, templates,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
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

/// Helper to get display name for navbar from CookieUser
fn get_navbar_display_name(user: &CookieUser) -> String {
    user.display_name()
}

pub async fn home_page(
    user: CookieUser,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, StatusCode> {
    let stats = state.db.get_stats().await.map_err(|e| {
        tracing::error!("Failed to get stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let display_name = get_navbar_display_name(&user);
    let content = templates::home(&stats);
    let page = templates::base_with_user("Home", content, &display_name, user.role(), user.is_registered());

    Ok(Html(page.into_string()))
}

pub async fn map_page(
    user: CookieUser,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, StatusCode> {
    let locations = state.db.list_active_locations().await.map_err(|e| {
        tracing::error!("Failed to get active locations: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Compute balances for all locations
    let mut location_balances = Vec::new();
    for location in &locations {
        let pool_msats = state
            .db
            .get_location_donation_pool_balance(&location.id)
            .await
            .unwrap_or(0);
        let balance_msats = compute_balance_msats(
            pool_msats,
            location.last_withdraw_at,
            location.created_at,
            &state.balance_config,
        );
        location_balances.push((location, balance_msats / 1000, pool_msats / 1000));
    }

    let display_name = get_navbar_display_name(&user);
    let content = templates::map(&location_balances);
    let page = templates::base_with_user("Map", content, &display_name, user.role(), user.is_registered());

    Ok(Html(page.into_string()))
}

pub async fn new_location_page(user: CookieUser) -> Result<Html<String>, Response> {
    let username = user.ensure_registered_with_role(UserRole::Creator)?;
    let content = templates::new_location();
    let page = templates::base_with_user("Add Location", content, username, user.role(), true);
    Ok(Html(page.into_string()))
}

pub async fn location_detail_page(
    user: CookieUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<LocationDetailQuery>,
) -> Result<Html<String>, StatusCode> {
    let location = state
        .db
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

    let scans = state
        .db
        .get_scans_with_user_for_location(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get scans: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get location's donation pool balance and donation history
    let pool_msats = state
        .db
        .get_location_donation_pool_balance(&id)
        .await
        .unwrap_or(0);
    let donations = state
        .db
        .list_location_donations(&id)
        .await
        .unwrap_or_default();

    // Compute current balance
    let available_msats = compute_balance_msats(
        pool_msats,
        location.last_withdraw_at,
        location.created_at,
        &state.balance_config,
    );

    // Get NFC card for wipe QR code (for owner/admin)
    let nfc_card = state.db.get_nfc_card_by_location(&id).await.unwrap_or(None);

    let current_user_id = Some(user.user_id.as_str());
    let current_user_role = user.role();
    let display_name = get_navbar_display_name(&user);

    let content = templates::location_detail(
        &location,
        &photos,
        &scans,
        available_msats / 1000,
        pool_msats / 1000,
        current_user_id,
        current_user_role,
        params.error.as_deref(),
        params.success.as_deref(),
        params.amount,
        &state.base_url,
        &donations,
        nfc_card.as_ref(),
    );
    let page = templates::base_with_user(&location.name, content, &display_name, user.role(), user.is_registered());

    Ok(Html(page.into_string()))
}

pub async fn nfc_setup_page(
    user: CookieUser,
    State(state): State<Arc<AppState>>,
    Path(write_token): Path<String>,
) -> Result<Redirect, StatusCode> {
    let _username = user
        .ensure_registered()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Get location by write token and redirect to location detail page
    let location = state
        .db
        .get_location_by_write_token(&write_token)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get location by write token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Redirect to location detail page where NFC setup is now integrated
    Ok(Redirect::to(&format!("/locations/{}", location.id)))
}

pub async fn donate_page(
    user: CookieUser,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, StatusCode> {
    // Get total pool balance across all locations
    let locations = state.db.list_active_locations().await.map_err(|e| {
        tracing::error!("Failed to list locations: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut total_pool_msats = 0i64;
    for location in &locations {
        total_pool_msats += state
            .db
            .get_location_donation_pool_balance(&location.id)
            .await
            .unwrap_or(0);
    }

    let received_donations = state
        .db
        .list_all_received_donations(50)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get received donations: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let display_name = get_navbar_display_name(&user);
    let content = templates::donate(
        total_pool_msats / 1000,
        locations.len(),
        &received_donations,
    );
    let page = templates::base_with_user("Donate", content, &display_name, user.role(), user.is_registered());

    Ok(Html(page.into_string()))
}

pub async fn login_page(_user: CookieUser, Query(params): Query<ErrorQuery>) -> Html<String> {
    let content = templates::login(params.error.as_deref());
    let page = templates::base("Login", content);
    Html(page.into_string())
}

pub async fn register_page(_user: CookieUser, Query(params): Query<ErrorQuery>) -> Html<String> {
    let content = templates::register(params.error.as_deref());
    let page = templates::base("Register", content);
    Html(page.into_string())
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
    user: CookieUser,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, Response> {
    let _username = user.ensure_registered()?;

    // Get user data
    let db_user = state
        .db
        .get_user_by_id(&user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get user: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?
        .ok_or_else(|| {
            tracing::error!("User not found: {}", user.user_id);
            StatusCode::NOT_FOUND.into_response()
        })?;

    // Get user's locations
    let locations = state
        .db
        .get_locations_by_user(&user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get user locations: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

    // Compute balances for user's locations
    let mut location_balances = Vec::new();
    for location in &locations {
        let pool_msats = state
            .db
            .get_location_donation_pool_balance(&location.id)
            .await
            .unwrap_or(0);
        let balance_msats = compute_balance_msats(
            pool_msats,
            location.last_withdraw_at,
            location.created_at,
            &state.balance_config,
        );
        location_balances.push((location, balance_msats / 1000, pool_msats / 1000));
    }

    let content = templates::profile(&db_user, &location_balances);
    let display_name = db_user.display_name();
    let page = templates::base_with_user("Profile", content, &display_name, user.role(), user.is_registered());

    Ok(Html(page.into_string()))
}

/// Withdraw/Collection page - displays the collection UI for the custodial wallet.
///
/// The URL should contain picc_data and cmac query parameters from the NFC chip
/// for counter verification.
///
/// On first scan of a programmed location, this activates the location and
/// redirects to the location details page.
pub async fn withdraw_page(
    user: CookieUser,
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
    Query(params): Query<WithdrawQuery>,
) -> Result<Response, StatusCode> {
    // Get the location
    let location = state
        .db
        .get_location(&location_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get user's current balance (if they have any transactions)
    let user_balance_msats = state.db.get_user_balance(&user.user_id).await.unwrap_or(0);
    let user_balance_sats = user_balance_msats / 1000;

    // Compute the location's available balance
    let pool_msats = state
        .db
        .get_location_donation_pool_balance(&location_id)
        .await
        .unwrap_or(0);
    let available_msats = compute_balance_msats(
        pool_msats,
        location.last_withdraw_at,
        location.created_at,
        &state.balance_config,
    );
    let available_sats = available_msats / 1000;

    // Get user info from DB for template
    let db_user = state.db.get_user_by_id(&user.user_id).await.ok().flatten();

    let is_new_user = matches!(user.kind, UserKind::AnonNew);
    let display_name = get_navbar_display_name(&user);

    // Check if we have SUN parameters
    let (picc_data, cmac) = match (&params.p, &params.c) {
        (Some(p), Some(c)) => (p.clone(), c.clone()),
        _ => {
            // No SUN parameters - show error
            let content = templates::collect(templates::CollectParams {
                location: &location,
                available_sats,
                current_balance_sats: user_balance_sats,
                scan_id: None,
                error: Some("Invalid NFC scan. Please scan the sticker again."),
                is_new_user,
                user: db_user.as_ref(),
            });
            let page = templates::base_with_user(
                "Collect Sats",
                content,
                &display_name,
                user.role(),
                user.is_registered(),
            );
            return Ok(Html(page.into_string()).into_response());
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
                state
                    .db
                    .update_nfc_card_counter(&v.nfc_card.location_id, v.counter as i64)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to update NFC card counter: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                // Activate the location
                state
                    .db
                    .update_location_status(&location.id, "active")
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to activate location: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                tracing::info!("Location {} activated on first scan", location.name);

                // Redirect to location details page
                return Ok(
                    Redirect::to(&format!("/locations/{}?success=activated", location.id))
                        .into_response(),
                );
            }
            Err(e) => {
                tracing::error!("First scan verification failed: {}", e);
                // Fall through to show error on collection page
            }
        }
    }

    // For active locations, record the scan and show the claim page
    match verification {
        Ok(v) => {
            // Record the scan (updates counter atomically)
            match state
                .db
                .record_nfc_scan(&location_id, &user.user_id, v.counter as i64)
                .await
            {
                Ok(Some(scan)) => {
                    // Success - show claim page with scan info
                    let content = templates::collect(templates::CollectParams {
                        location: &location,
                        available_sats,
                        current_balance_sats: user_balance_sats,
                        scan_id: Some(&scan.id),
                        error: params.error.as_deref(),
                        is_new_user,
                        user: db_user.as_ref(),
                    });
                    let page = templates::base_with_user(
                        "Collect Sats",
                        content,
                        &display_name,
                        user.role(),
                        user.is_registered(),
                    );
                    Ok(Html(page.into_string()).into_response())
                }
                Ok(None) => {
                    // Counter already used (race condition or replay)
                    let content = templates::collect(templates::CollectParams {
                        location: &location,
                        available_sats,
                        current_balance_sats: user_balance_sats,
                        scan_id: None,
                        error: Some(
                            "This scan has already been used. Please scan the sticker again.",
                        ),
                        is_new_user,
                        user: db_user.as_ref(),
                    });
                    let page = templates::base_with_user(
                        "Collect Sats",
                        content,
                        &display_name,
                        user.role(),
                        user.is_registered(),
                    );
                    Ok(Html(page.into_string()).into_response())
                }
                Err(e) => {
                    tracing::error!("Failed to record scan: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(ntag424::SunError::ReplayDetected { .. }) => {
            // Check if user has an existing valid scan they can still claim
            let existing_scan = state
                .db
                .get_last_scan_for_location(&location_id)
                .await
                .ok()
                .flatten();
            let (scan_id, error) = match existing_scan {
                Some(scan) if scan.user_id == user.user_id && scan.is_claimable() => {
                    // User's own recent scan - they can still claim
                    (Some(scan.id), None)
                }
                _ => (
                    None,
                    Some("This scan has already been used. Please scan the sticker again."),
                ),
            };
            let content = templates::collect(templates::CollectParams {
                location: &location,
                available_sats,
                current_balance_sats: user_balance_sats,
                scan_id: scan_id.as_deref(),
                error,
                is_new_user,
                user: db_user.as_ref(),
            });
            let page = templates::base_with_user(
                "Collect Sats",
                content,
                &display_name,
                user.role(),
                user.is_registered(),
            );
            Ok(Html(page.into_string()).into_response())
        }
        Err(e) => {
            let error_message = match e {
                ntag424::SunError::CmacMismatch => {
                    "Invalid NFC scan. Please scan the sticker again."
                }
                ntag424::SunError::UidMismatch { .. } => {
                    "Invalid NFC card. This card is not associated with this location."
                }
                ntag424::SunError::CardNotFound | ntag424::SunError::CardNotProgrammed => {
                    "NFC card not configured. Please contact the location owner."
                }
                _ => {
                    tracing::error!("SUN verification error: {}", e);
                    "Verification failed. Please try scanning again."
                }
            };
            let content = templates::collect(templates::CollectParams {
                location: &location,
                available_sats,
                current_balance_sats: user_balance_sats,
                scan_id: None,
                error: Some(error_message),
                is_new_user,
                user: db_user.as_ref(),
            });
            let page = templates::base_with_user(
                "Collect Sats",
                content,
                &display_name,
                user.role(),
                user.is_registered(),
            );
            Ok(Html(page.into_string()).into_response())
        }
    }
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
    user: CookieUser,
    State(state): State<Arc<AppState>>,
    Query(params): Query<WalletQuery>,
) -> Html<String> {
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

    // Generate LNURL-withdraw string if user has balance
    let lnurlw_string = if balance_sats > 0 {
        // Create a signed token for authentication (valid for 1 hour)
        let token = create_withdraw_token(&state.withdraw_secret, &user.user_id);
        let lnurlw_url = format!(
            "{}/api/wallet/lnurlw?token={}",
            state.base_url,
            urlencoding::encode(&token)
        );
        crate::lnurl::encode_lnurl(&lnurlw_url).ok()
    } else {
        None
    };

    // Build content
    let content = templates::wallet(
        balance_sats,
        &transactions,
        db_user.as_ref(),
        params.success.as_deref(),
        params.amount,
        params.location.as_deref(),
        lnurlw_string.as_deref(),
    );
    let display_name = get_navbar_display_name(&user);
    let page = templates::base_with_user("My Wallet", content, &display_name, user.role(), user.is_registered());

    Html(page.into_string())
}

/// Admin users page - allows admins to manage user roles
pub async fn admin_users_page(
    user: CookieUser,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, Response> {
    // Require admin role
    let username = user.ensure_registered_with_role(UserRole::Admin)?;

    // Get all users
    let users = state.db.list_users().await.map_err(|e| {
        tracing::error!("Failed to list users: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    let content = templates::admin_users(&users);
    let page = templates::base_with_user("User Management", content, username, user.role(), true);

    Ok(Html(page.into_string()))
}

/// Admin locations page - allows admins to view and manage all locations
pub async fn admin_locations_page(
    user: CookieUser,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, Response> {
    // Require admin role
    let username = user.ensure_registered_with_role(UserRole::Admin)?;

    // Get all locations
    let locations = state.db.list_locations().await.map_err(|e| {
        tracing::error!("Failed to list locations: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    // Compute balances for all locations
    let mut location_balances = Vec::new();
    for location in &locations {
        let pool_msats = state
            .db
            .get_location_donation_pool_balance(&location.id)
            .await
            .unwrap_or(0);
        let balance_msats = compute_balance_msats(
            pool_msats,
            location.last_withdraw_at,
            location.created_at,
            &state.balance_config,
        );
        location_balances.push((location, balance_msats / 1000, pool_msats / 1000));
    }

    let content = templates::admin_locations(&location_balances);
    let page =
        templates::base_with_user("Location Management", content, username, user.role(), true);

    Ok(Html(page.into_string()))
}
