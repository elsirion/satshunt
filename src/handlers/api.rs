use crate::{
    auth::{AuthUser, CookieUser, Key, RequireRegistered},
    balance::BalanceConfig,
    db::Database,
    donation::NewDonation,
    lightning::{Lightning, LightningService},
    lnurl,
    models::{ClaimResult, UserRole},
    ntag424,
};
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    Form,
};
use axum_extra::extract::cookie::PrivateCookieJar;
use chrono::Utc;
use hmac::{Hmac, Mac};
use image::{DynamicImage, GenericImageView};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::Sha256;
use std::{path::PathBuf, sync::Arc};
use tokio::fs;
use tokio::sync::mpsc;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Deserialize)]
pub struct CreateLocationRequest {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub description: Option<String>,
}

pub struct AppState {
    pub db: Database,
    pub lightning: Arc<dyn Lightning>,
    pub upload_dir: PathBuf,
    pub base_url: String,
    pub balance_config: BalanceConfig,
    pub donation_sender: mpsc::UnboundedSender<NewDonation>,
    /// Key for signing private cookies
    pub cookie_key: Key,
    /// Secret for signing withdrawal tokens (derived from cookie_key)
    pub withdraw_secret: Vec<u8>,
}

/// Calculate Lightning network fees for a withdrawal.
/// Returns (max_withdrawable_msats, fee_msats) given a balance in msats.
/// Fee structure: 2 sats fixed + 0.5% routing fee
fn calculate_withdrawal_fees(balance_msats: i64) -> (i64, i64) {
    let routing_fee_msats = ((balance_msats as f64) * 0.005).ceil() as i64;
    let fixed_fee_msats = 2000; // 2 sats
    let total_fee_msats = routing_fee_msats + fixed_fee_msats;
    let max_withdrawable_msats = (balance_msats - total_fee_msats).max(0);
    (max_withdrawable_msats, total_fee_msats)
}

/// Check if an invoice amount plus fees fits within balance.
/// Returns Ok(fee_msats) if valid, or Err with error message.
fn check_invoice_with_fees(invoice_msats: i64, balance_msats: i64) -> Result<i64, String> {
    // Calculate fee for this specific invoice amount
    let routing_fee_msats = ((invoice_msats as f64) * 0.005).ceil() as i64;
    let fixed_fee_msats = 2000; // 2 sats
    let total_fee_msats = routing_fee_msats + fixed_fee_msats;
    let total_required_msats = invoice_msats + total_fee_msats;

    if total_required_msats > balance_msats {
        let max_after_fees = calculate_withdrawal_fees(balance_msats).0 / 1000;
        Err(format!(
            "Invoice ({} sats) + fees ({} sats) exceeds balance. Max withdrawal: {} sats.",
            invoice_msats / 1000,
            total_fee_msats / 1000,
            max_after_fees
        ))
    } else {
        Ok(total_fee_msats)
    }
}

/// Create a signed withdrawal token for a user.
/// Format: "{user_id}:{timestamp}:{signature}"
/// The token is valid for 1 hour.
pub fn create_withdraw_token(secret: &[u8], user_id: &str) -> String {
    let timestamp = Utc::now().timestamp();
    let message = format!("{}:{}", user_id, timestamp);

    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    format!("{}:{}:{}", user_id, timestamp, signature)
}

/// Verify a withdrawal token and extract the user_id.
/// Returns None if invalid or expired (> 1 hour old).
pub fn verify_withdraw_token(secret: &[u8], token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split(':').collect();
    if parts.len() != 3 {
        tracing::warn!("Invalid withdraw token format");
        return None;
    }

    let user_id = parts[0];
    let timestamp: i64 = parts[1].parse().ok()?;
    let provided_signature = parts[2];

    // Check token age (1 hour max)
    let now = Utc::now().timestamp();
    let age_seconds = now - timestamp;
    if !(0..=3600).contains(&age_seconds) {
        tracing::warn!("Withdraw token expired (age: {} seconds)", age_seconds);
        return None;
    }

    // Verify signature
    let message = format!("{}:{}", user_id, timestamp);
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let expected_signature = hex::encode(mac.finalize().into_bytes());

    if expected_signature != provided_signature {
        tracing::warn!("Invalid withdraw token signature");
        return None;
    }

    Some(user_id.to_string())
}

pub async fn create_location(
    auth: RequireRegistered,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateLocationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // Require Creator role to create locations
    auth.ensure_role(UserRole::Creator)
        .map_err(|_| StatusCode::FORBIDDEN)?;

    tracing::info!(
        "Creating location: {} at ({}, {})",
        payload.name,
        payload.latitude,
        payload.longitude
    );

    // Generate LNURL secret
    let lnurlw_secret = LightningService::generate_lnurlw_secret();

    // Create location in database
    let location = state
        .db
        .create_location(
            payload.name,
            payload.latitude,
            payload.longitude,
            payload.description,
            lnurlw_secret,
            auth.user_id,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to create location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!({
        "location_id": location.id,
        "write_token": location.write_token
    })))
}

pub async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let stats = state.db.get_stats().await.map_err(|e| {
        tracing::error!("Failed to get stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!(stats)))
}

#[derive(serde::Deserialize)]
pub struct DonationInvoiceRequest {
    pub amount: i64,
    /// Optional location ID for direct location donations.
    /// If None, the donation goes to the global pool (split among all locations).
    pub location_id: Option<String>,
}

/// Generate a Lightning invoice for donation
pub async fn create_donation_invoice(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DonationInvoiceRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if payload.amount <= 0 {
        tracing::error!("Invalid donation amount: {}", payload.amount);
        return Err(StatusCode::BAD_REQUEST);
    }

    // If location_id is provided, verify it exists
    let location_name = if let Some(ref loc_id) = payload.location_id {
        let location = state
            .db
            .get_location(loc_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get location: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or_else(|| {
                tracing::error!("Location not found: {}", loc_id);
                StatusCode::NOT_FOUND
            })?;
        Some(location.name)
    } else {
        None
    };

    let description = if let Some(ref name) = location_name {
        tracing::info!(
            "Creating invoice for {} sats donation to location '{}'",
            payload.amount,
            name
        );
        format!("SatsHunt donation to '{}': {} sats", name, payload.amount)
    } else {
        tracing::info!(
            "Creating invoice for {} sats global donation",
            payload.amount
        );
        format!("SatsHunt donation: {} sats", payload.amount)
    };

    // Generate Lightning invoice
    let invoice = state
        .lightning
        .create_invoice(payload.amount as u64, &description)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create invoice: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let amount_msats = payload.amount * 1000;

    // Store donation in database for resilient tracking
    state
        .db
        .create_donation(
            invoice.clone(),
            amount_msats,
            payload.location_id.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to create donation: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Notify donation service to start awaiting payment
    if let Err(e) = state.donation_sender.send(NewDonation {
        invoice: invoice.clone(),
        amount_msats,
        location_id: payload.location_id.clone(),
    }) {
        tracing::error!("Failed to notify donation service: {}", e);
        // Don't fail the request - the donation service will pick it up on next restart
    }

    // Generate QR code
    use image::Luma;
    use qrcode::QrCode;

    let qr_code = QrCode::new(&invoice).map_err(|e| {
        tracing::error!("Failed to create QR code: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let qr_image = qr_code.render::<Luma<u8>>().build();

    // Convert to PNG bytes
    let mut png_bytes = Vec::new();
    use image::codecs::png::PngEncoder;
    use image::{ExtendedColorType, ImageEncoder};

    let encoder = PngEncoder::new(&mut png_bytes);
    encoder
        .write_image(
            qr_image.as_raw(),
            qr_image.width(),
            qr_image.height(),
            ExtendedColorType::L8,
        )
        .map_err(|e| {
            tracing::error!("Failed to encode QR code as PNG: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Encode as base64
    use base64::Engine;
    let qr_base64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

    tracing::info!("Invoice created and pending donation recorded");

    Ok(Json(json!({
        "invoice": invoice,
        "qr_code": format!("data:image/png;base64,{}", qr_base64),
        "amount": payload.amount
    })))
}

/// Wait for invoice payment by polling the database.
/// This is resilient against client disconnects - the background DonationService
/// handles the actual payment detection independently.
pub async fn wait_for_donation(
    State(state): State<Arc<AppState>>,
    Path(invoice_and_amount): Path<String>,
) -> Result<axum::response::Html<String>, StatusCode> {
    // Invoice format: {invoice_string}:{amount}:{prefix}
    let parts: Vec<&str> = invoice_and_amount.split(':').collect();
    if parts.len() != 3 {
        tracing::error!("Invalid invoice format");
        return Err(StatusCode::BAD_REQUEST);
    }

    let invoice = parts[0];
    let amount: i64 = parts[1].parse().map_err(|_| {
        tracing::error!("Invalid amount in path");
        StatusCode::BAD_REQUEST
    })?;
    let prefix = parts[2];

    tracing::info!("Polling for payment of {} sats invoice", amount);

    // Poll the database for up to 5 minutes (300 seconds) with 2-second intervals
    const MAX_POLLS: u32 = 150;
    const POLL_INTERVAL_MS: u64 = 2000;

    for poll in 0..MAX_POLLS {
        // Check if the donation is received
        match state.db.get_donation_by_invoice(invoice).await {
            Ok(Some(donation)) if donation.is_received() => {
                tracing::info!("Payment confirmed for {} sats donation", amount);

                // Generate response based on whether it's a location or global donation
                let html = if let Some(ref loc_id) = donation.location_id {
                    // Location-specific donation
                    let location = state.db.get_location(loc_id).await.map_err(|e| {
                        tracing::error!("Failed to get location: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                    if let Some(location) = location {
                        // Get pool balance from the donations table
                        let pool_balance_msats = state
                            .db
                            .get_location_donation_pool_balance(loc_id)
                            .await
                            .unwrap_or(0);

                        format!(
                            r#"<div class="p-6" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);">
                                <div class="p-3 flex items-center gap-2" style="background: rgba(107, 155, 107, 0.25); border: 2px solid var(--color-success);">
                                    <i class="fa-solid fa-check-circle" style="color: var(--color-success);"></i>
                                    <span class="text-sm font-bold text-primary">Payment received! Thank you for donating {} sats to '{}'!</span>
                                </div>
                                <div class="text-center mt-4">
                                    <p class="text-sm text-muted font-bold">{}'s Donation Pool</p>
                                    <p class="text-3xl font-black text-highlight orange">{} <i class="fa-solid fa-bolt"></i></p>
                                </div>
                                <button type="button" onclick="reset{}Donation()" class="btn-brutal mt-4 w-full">Done</button>
                            </div>"#,
                            amount,
                            location.name,
                            location.name,
                            pool_balance_msats / 1000,
                            if prefix.is_empty() { "" } else { "Location" }
                        )
                    } else {
                        // Location was deleted, fall back to generic message
                        format!(
                            r#"<div class="p-6" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);">
                                <div class="p-3 flex items-center gap-2" style="background: rgba(107, 155, 107, 0.25); border: 2px solid var(--color-success);">
                                    <i class="fa-solid fa-check-circle" style="color: var(--color-success);"></i>
                                    <span class="text-sm font-bold text-primary">Payment received! Thank you for donating {} sats!</span>
                                </div>
                                <button type="button" onclick="reset{}Donation()" class="btn-brutal mt-4 w-full">Done</button>
                            </div>"#,
                            amount,
                            if prefix.is_empty() { "" } else { "Location" }
                        )
                    }
                } else {
                    // Global donation - was split among all locations
                    let locations = state.db.list_active_locations().await.map_err(|e| {
                        tracing::error!("Failed to list locations: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                    let num_locations = locations.len();
                    let per_location = if num_locations > 0 {
                        amount / num_locations as i64
                    } else {
                        0
                    };

                    format!(
                        r#"<div class="p-6" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);">
                            <div class="p-3 flex items-center gap-2" style="background: rgba(107, 155, 107, 0.25); border: 2px solid var(--color-success);">
                                <i class="fa-solid fa-check-circle" style="color: var(--color-success);"></i>
                                <span class="text-sm font-bold text-primary">Payment received! Thank you for donating {} sats!</span>
                            </div>
                            <div class="text-center mt-4">
                                <p class="text-sm text-muted font-bold">Split Among {} Locations</p>
                                <p class="text-3xl font-black text-highlight orange">{} <i class="fa-solid fa-bolt"></i> each</p>
                            </div>
                            <button type="button" onclick="reset{}Donation()" class="btn-brutal mt-4 w-full">Done</button>
                        </div>"#,
                        amount,
                        num_locations,
                        per_location,
                        if prefix.is_empty() { "" } else { "Location" }
                    )
                };

                return Ok(axum::response::Html(html));
            }
            Ok(Some(_)) => {
                // Still pending, continue polling
                if poll % 15 == 0 {
                    // Log every 30 seconds
                    tracing::debug!("Still waiting for payment... (poll {})", poll);
                }
            }
            Ok(None) => {
                tracing::error!("Pending donation not found for invoice");
                return Err(StatusCode::NOT_FOUND);
            }
            Err(e) => {
                tracing::error!("Failed to check pending donation: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
    }

    // Timeout - return a message asking user to check later
    // The payment might still come through and will be credited by the background service
    let html = format!(
        r#"<div class="p-6" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);">
            <div class="p-3 flex items-center gap-2" style="background: rgba(181, 152, 107, 0.25); border: 2px solid var(--color-warning);">
                <i class="fa-solid fa-hourglass-half" style="color: var(--color-warning);"></i>
                <span class="text-sm font-bold text-primary">Still waiting for payment...</span>
            </div>
            <p class="text-sm text-secondary mt-3">The invoice is still valid. If you've already paid, your donation will be credited shortly.</p>
            <p class="text-sm text-muted mt-1">You can safely close this page - the payment will be processed automatically.</p>
            <button type="button" onclick="reset{}Donation()" class="btn-brutal mt-4 w-full">Close</button>
        </div>"#,
        if prefix.is_empty() { "" } else { "Location" }
    );

    Ok(axum::response::Html(html))
}

/// Generate a random 32-character hex string for card keys
fn generate_card_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    hex::encode(bytes)
}

#[derive(Debug, Deserialize)]
pub struct BoltcardKeysRequest {
    #[serde(rename = "UID")]
    uid: Option<String>,
    #[serde(rename = "LNURLW")]
    lnurlw: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BoltcardKeysResponse {
    #[serde(rename = "LNURLW")]
    lnurlw: String,
    #[serde(rename = "K0")]
    k0: String,
    #[serde(rename = "K1")]
    k1: String,
    #[serde(rename = "K2")]
    k2: String,
    #[serde(rename = "K3")]
    k3: String,
    #[serde(rename = "K4")]
    k4: String,
}

/// Boltcard NFC Programmer keys endpoint
/// This endpoint is called by the Boltcard NFC Programmer app to get card keys
/// It handles both program (UID) and reset (LNURLW) actions
pub async fn boltcard_keys(
    State(state): State<Arc<AppState>>,
    Path(write_token): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    Json(payload): Json<BoltcardKeysRequest>,
) -> Result<Json<BoltcardKeysResponse>, StatusCode> {
    tracing::info!("Boltcard keys request for token: {}", write_token);
    tracing::debug!("Query params: {:?}", params);
    tracing::debug!("Payload: {:?}", payload);

    let on_existing = params.get("onExisting").map(|s| s.as_str());

    // Get location by ID (the path parameter is the location ID)
    let location = state
        .db
        .get_location(&write_token)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Location not found: {}", write_token);
            StatusCode::NOT_FOUND
        })?;

    tracing::info!("Found location: {} ({})", location.name, location.id);

    // Check if we already have an NFC card for this location
    let mut existing_card = state
        .db
        .get_nfc_card_by_location(&location.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get NFC card: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let lnurlw_url = format!("{}/withdraw/{}", state.base_url, location.id);

    // Handle program action (UID provided)
    if let Some(uid) = &payload.uid {
        tracing::info!("Program action for UID: {}", uid);

        if existing_card.is_none() {
            // Create new NFC card with generated keys
            tracing::info!("Creating new NFC card for location");

            let k0 = generate_card_key();
            let k1 = generate_card_key();
            let k2 = generate_card_key();
            let k3 = generate_card_key();
            let k4 = generate_card_key();

            let card = state
                .db
                .create_nfc_card(location.id.clone(), k0, k1, k2, k3, k4)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to create NFC card: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            existing_card = Some(card);
        } else {
            // Card exists - reset counter for reprogramming
            tracing::info!("Reprogramming existing card, resetting counter");
            state
                .db
                .reset_nfc_card_counter(&location.id)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to reset counter: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            // Fetch updated card
            existing_card = state
                .db
                .get_nfc_card_by_location(&location.id)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to get NFC card: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }

        // Update UID and mark as programmed
        state
            .db
            .update_nfc_card_uid_and_mark_programmed(&location.id, uid)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update UID: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Mark location as programmed (but don't mark token as used yet - allow retries)
        state
            .db
            .update_location_status(&location.id, "programmed")
            .await
            .map_err(|e| {
                tracing::error!("Failed to update location status: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        tracing::info!(
            "Location {} marked as programmed (write token still valid for retries)",
            location.name
        );
    }
    // Handle reset action (LNURLW provided)
    else if let Some(lnurlw) = &payload.lnurlw {
        tracing::info!("Reset action for LNURLW: {}", lnurlw);

        // Verify the LNURLW matches this location
        if !lnurlw.contains(&location.id) {
            tracing::warn!("LNURLW does not match location");
            return Err(StatusCode::BAD_REQUEST);
        }

        if existing_card.is_none() {
            tracing::warn!("No card exists to reset");
            return Err(StatusCode::NOT_FOUND);
        }

        match on_existing {
            Some("UpdateVersion") => {
                tracing::info!("Incrementing version on reset");
                state
                    .db
                    .increment_nfc_card_version(&location.id)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to increment version: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                // Fetch updated card
                existing_card = state
                    .db
                    .get_nfc_card_by_location(&location.id)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to get updated NFC card: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;
            }
            _ => {
                tracing::info!("Keeping version on reset");
            }
        }
    } else {
        tracing::error!("Neither UID nor LNURLW provided");
        return Err(StatusCode::BAD_REQUEST);
    }

    let card = existing_card.ok_or_else(|| {
        tracing::error!("Card should exist at this point");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Returning keys for card (version: {})", card.version);

    Ok(Json(BoltcardKeysResponse {
        lnurlw: lnurlw_url,
        k0: card.k0_auth_key,
        k1: card.k1_decrypt_key,
        k2: card.k2_cmac_key,
        k3: card.k3,
        k4: card.k4,
    }))
}

/// Delete a non-active location (created or programmed only)
pub async fn delete_location(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
    auth: AuthUser,
) -> Result<StatusCode, StatusCode> {
    tracing::info!(
        "Delete request for location {} by user {}",
        location_id,
        auth.user_id
    );

    // First check if location exists and belongs to user
    let location = state
        .db
        .get_location(&location_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Location not found: {}", location_id);
            StatusCode::NOT_FOUND
        })?;

    // Check ownership
    if location.user_id != auth.user_id {
        tracing::warn!(
            "User {} attempted to delete location {} owned by {}",
            auth.user_id,
            location_id,
            location.user_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if active (cannot delete active locations)
    if location.is_active() {
        tracing::warn!(
            "User {} attempted to delete active location {}",
            auth.user_id,
            location_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Log deletion - any pool balance remains but is no longer accessible
    tracing::info!("Location {} ({}) deleted", location.name, location_id);

    // Delete the location
    let result = state
        .db
        .delete_location(&location_id, &auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        tracing::warn!(
            "Location {} not deleted (may have been activated or doesn't exist)",
            location_id
        );
        return Err(StatusCode::NOT_FOUND);
    }

    tracing::info!(
        "Location {} deleted by user {}",
        location.name,
        auth.user_id
    );
    Ok(StatusCode::NO_CONTENT)
}

/// Apply EXIF orientation to correctly rotate images from cameras/phones
fn apply_exif_orientation(data: &[u8], img: DynamicImage) -> DynamicImage {
    let orientation = (|| {
        let mut cursor = std::io::Cursor::new(data);
        let exif_reader = exif::Reader::new();
        let exif = exif_reader.read_from_container(&mut cursor).ok()?;
        let orientation = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)?;
        orientation.value.get_uint(0)
    })();

    match orientation {
        Some(2) => img.fliph(),
        Some(3) => img.rotate180(),
        Some(4) => img.flipv(),
        Some(5) => img.rotate90().fliph(),
        Some(6) => img.rotate90(),
        Some(7) => img.rotate270().fliph(),
        Some(8) => img.rotate270(),
        _ => img, // 1 or unknown = no rotation needed
    }
}

/// Upload a photo to a location
pub async fn upload_photo(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
    mut multipart: Multipart,
) -> Result<StatusCode, StatusCode> {
    tracing::info!(
        "Photo upload request for location {} by user {}",
        location_id,
        auth.user_id
    );

    // Get location and verify ownership
    let location = state
        .db
        .get_location(&location_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Location not found: {}", location_id);
            StatusCode::NOT_FOUND
        })?;

    // Check ownership
    if location.user_id != auth.user_id {
        tracing::warn!(
            "User {} attempted to upload photo to location {} owned by {}",
            auth.user_id,
            location_id,
            location.user_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if location is active (cannot modify photos of active locations)
    if location.is_active() {
        tracing::warn!(
            "User {} attempted to upload photo to active location {}",
            auth.user_id,
            location_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Process uploaded photo
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::error!("Failed to read multipart field: {}", e);
        StatusCode::BAD_REQUEST
    })? {
        if field.name() == Some("photo") {
            let data = field.bytes().await.map_err(|e| {
                tracing::error!("Failed to read photo data: {}", e);
                StatusCode::BAD_REQUEST
            })?;

            // Decode image to validate it's a real image
            let img = image::load_from_memory(&data).map_err(|e| {
                tracing::error!("Failed to decode image: {}", e);
                StatusCode::BAD_REQUEST
            })?;

            // Apply EXIF orientation to fix rotated images
            let img = apply_exif_orientation(&data, img);

            // Resize if larger than 12 megapixels
            const MAX_PIXELS: u32 = 12_000_000;
            let (width, height) = img.dimensions();
            let total_pixels = width as u64 * height as u64;

            let img = if total_pixels > MAX_PIXELS as u64 {
                let scale = ((MAX_PIXELS as f64) / (total_pixels as f64)).sqrt();
                let new_width = (width as f64 * scale) as u32;
                let new_height = (height as f64 * scale) as u32;

                tracing::info!(
                    "Resizing image from {}x{} to {}x{}",
                    width,
                    height,
                    new_width,
                    new_height
                );

                img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
            } else {
                img
            };

            // Generate clean UUID filename
            let filename = format!("{}.jpg", uuid::Uuid::new_v4());
            let file_path = state.upload_dir.join(&filename);

            // Encode as JPEG and save
            img.save_with_format(&file_path, image::ImageFormat::Jpeg)
                .map_err(|e| {
                    tracing::error!("Failed to save JPEG: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            state
                .db
                .add_photo(&location_id, filename)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to save photo record: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            tracing::info!(
                "Photo uploaded and converted successfully for location {}",
                location.name
            );
            return Ok(StatusCode::OK);
        }
    }

    Err(StatusCode::BAD_REQUEST)
}

/// Delete a photo
pub async fn delete_photo(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(photo_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!(
        "Photo delete request for photo {} by user {}",
        photo_id,
        auth.user_id
    );

    // Get photo to verify it exists and get location_id
    let photo = state
        .db
        .get_photo(&photo_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get photo: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Photo not found: {}", photo_id);
            StatusCode::NOT_FOUND
        })?;

    // Get location to verify ownership
    let location = state
        .db
        .get_location(&photo.location_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Location not found: {}", photo.location_id);
            StatusCode::NOT_FOUND
        })?;

    // Check ownership
    if location.user_id != auth.user_id {
        tracing::warn!(
            "User {} attempted to delete photo from location {} owned by {}",
            auth.user_id,
            location.id,
            location.user_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if location is active (cannot modify photos of active locations)
    if location.is_active() {
        tracing::warn!(
            "User {} attempted to delete photo from active location {}",
            auth.user_id,
            location.id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Delete photo file
    let file_path = state.upload_dir.join(&photo.file_path);
    if file_path.exists() {
        fs::remove_file(&file_path).await.map_err(|e| {
            tracing::error!("Failed to delete photo file: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    // Delete photo record
    state.db.delete_photo(&photo_id).await.map_err(|e| {
        tracing::error!("Failed to delete photo record: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Photo {} deleted successfully", photo_id);
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Withdrawal API Endpoints
// ============================================================================

/// Query parameters for SUN message verification
#[derive(Debug, Deserialize)]
pub struct SunParams {
    /// NTAG424 encrypted picc_data (parameter name: p)
    #[serde(alias = "picc_data")]
    pub p: String,
    /// NTAG424 CMAC signature (parameter name: c)
    #[serde(alias = "cmac")]
    pub c: String,
}

/// Request body for LN address withdrawal
#[derive(Debug, Deserialize)]
pub struct LnAddressWithdrawRequest {
    pub ln_address: String,
}

/// Request body for invoice withdrawal
#[derive(Debug, Deserialize)]
pub struct InvoiceWithdrawRequest {
    pub invoice: String,
}

/// Response for withdrawal endpoints
#[derive(Debug, Serialize)]
pub struct WithdrawResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_sats: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl WithdrawResponse {
    fn success(amount_sats: i64, location_id: &str) -> Self {
        Self {
            success: true,
            amount_sats: Some(amount_sats),
            redirect_url: Some(format!(
                "/locations/{}?success=withdrawn&amount={}",
                location_id, amount_sats
            )),
            error: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            amount_sats: None,
            redirect_url: None,
            error: Some(message.into()),
        }
    }
}

/// Helper to verify SUN message and prepare for withdrawal
async fn verify_and_prepare_withdrawal(
    state: &AppState,
    location_id: &str,
    sun_params: &SunParams,
) -> Result<(crate::models::Location, crate::models::NfcCard, u32, i64), WithdrawResponse> {
    // Verify the SUN message
    let verification =
        ntag424::verify_sun_message(&state.db, location_id, &sun_params.p, &sun_params.c)
            .await
            .map_err(|e| match e {
                ntag424::SunError::ReplayDetected { .. } => WithdrawResponse::error(
                    "This scan has already been used. Please scan the sticker again.",
                ),
                ntag424::SunError::CmacMismatch => {
                    WithdrawResponse::error("Invalid NFC scan. Please scan the sticker again.")
                }
                ntag424::SunError::UidMismatch { .. } => {
                    WithdrawResponse::error("Invalid NFC card for this location.")
                }
                ntag424::SunError::CardNotFound | ntag424::SunError::CardNotProgrammed => {
                    WithdrawResponse::error("NFC card not configured.")
                }
                _ => {
                    tracing::error!("SUN verification error: {}", e);
                    WithdrawResponse::error("Verification failed. Please try again.")
                }
            })?;

    let location = verification.location;
    let nfc_card = verification.nfc_card;
    let counter = verification.counter;

    // Compute the actual withdrawable balance from pool and time
    let pool_balance_msats = state
        .db
        .get_location_donation_pool_balance(&location.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get pool balance: {}", e);
            WithdrawResponse::error("Failed to check balance.")
        })?;

    let withdrawable_msats = crate::balance::compute_balance_msats(
        pool_balance_msats,
        location.last_withdraw_at,
        location.created_at,
        &state.balance_config,
    );

    if withdrawable_msats <= 0 {
        return Err(WithdrawResponse::error(
            "No sats available at this location.",
        ));
    }

    Ok((location, nfc_card, counter, withdrawable_msats))
}

/// Record a successful withdrawal claim (called after payment succeeds)
/// Note: Legacy withdrawal methods don't track user_id, so we pass None
async fn record_withdrawal(state: &AppState, location_id: &str, amount_msats: i64) {
    // Record the claim - this is best-effort, payment already succeeded
    if let Err(e) = state.db.record_claim(location_id, amount_msats, None).await {
        tracing::error!("Failed to record claim: {}", e);
    }
}

/// Withdraw via Lightning Address
///
/// POST /api/withdraw/{location_id}/ln-address?picc_data={}&cmac={}
/// Body: { "ln_address": "user@domain.com" }
pub async fn withdraw_ln_address(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
    Query(sun_params): Query<SunParams>,
    Json(payload): Json<LnAddressWithdrawRequest>,
) -> Result<Json<WithdrawResponse>, StatusCode> {
    tracing::info!(
        "LN address withdrawal request for location {}: {}",
        location_id,
        payload.ln_address
    );

    // Verify SUN and get withdrawal info
    let (location, _nfc_card, counter, withdrawable_msats) =
        match verify_and_prepare_withdrawal(&state, &location_id, &sun_params).await {
            Ok(result) => result,
            Err(response) => return Ok(Json(response)),
        };

    let withdrawable_sats = withdrawable_msats / 1000;

    // Resolve LN address and get invoice (do this before claiming to avoid
    // claiming if the LN address is invalid)
    let invoice =
        match lnurl::get_invoice_for_ln_address(&payload.ln_address, withdrawable_msats).await {
            Ok(inv) => inv,
            Err(lnurl::LnurlError::InvalidFormat(msg)) => {
                return Ok(Json(WithdrawResponse::error(format!(
                    "Invalid Lightning address: {}",
                    msg
                ))));
            }
            Err(lnurl::LnurlError::AmountOutOfRange { min, max, .. }) => {
                return Ok(Json(WithdrawResponse::error(format!(
                    "Amount {} sats is outside the allowed range ({}-{} sats)",
                    withdrawable_sats,
                    min / 1000,
                    max / 1000
                ))));
            }
            Err(e) => {
                tracing::error!("LN address resolution failed: {}", e);
                return Ok(Json(WithdrawResponse::error(
                    "Could not resolve Lightning address. Please check and try again.",
                )));
            }
        };

    // Atomically claim the withdrawal (updates counter and zeros balance)
    // This prevents double-spending even if the same scan is used multiple times
    let claimed_msats = match state
        .db
        .claim_withdrawal(&location_id, counter as i64, &state.balance_config)
        .await
    {
        Ok(Some(msats)) => msats,
        Ok(None) => {
            return Ok(Json(WithdrawResponse::error(
                "This scan has already been used. Please scan the sticker again.",
            )));
        }
        Err(e) => {
            tracing::error!("Failed to claim withdrawal: {}", e);
            return Ok(Json(WithdrawResponse::error(
                "Failed to process withdrawal. Please try again.",
            )));
        }
    };

    let claimed_sats = claimed_msats / 1000;

    // Pay the invoice
    if let Err(e) = state.lightning.pay_invoice(&invoice).await {
        tracing::error!("Failed to pay invoice: {}", e);
        // Note: The balance was already claimed, so the user will need to scan again.
        // This is intentional to prevent double-spending attempts.
        return Ok(Json(WithdrawResponse::error(
            "Payment failed. Please scan the sticker again to retry.",
        )));
    }

    // Record the successful withdrawal
    record_withdrawal(&state, &location_id, claimed_msats).await;

    tracing::info!(
        "Successful LN address withdrawal from {}: {} sats to {}",
        location.name,
        claimed_sats,
        payload.ln_address
    );

    Ok(Json(WithdrawResponse::success(claimed_sats, &location_id)))
}

// ============================================================================
// LNURL-withdraw Endpoints (LUD-03)
// ============================================================================

/// LNURL-withdraw response (LUD-03)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LnurlWithdrawResponse {
    pub tag: String,
    pub callback: String,
    pub k1: String,
    pub default_description: String,
    pub min_withdrawable: i64,
    pub max_withdrawable: i64,
}

/// LNURL-withdraw callback response
#[derive(Debug, Serialize)]
pub struct LnurlCallbackResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl LnurlCallbackResponse {
    fn ok() -> Self {
        Self {
            status: "OK".to_string(),
            reason: None,
        }
    }

    fn error(reason: impl Into<String>) -> Self {
        Self {
            status: "ERROR".to_string(),
            reason: Some(reason.into()),
        }
    }
}

/// LNURL-withdraw initial request (LUD-03)
///
/// GET /api/lnurlw/{location_id}?picc_data={}&cmac={}
///
/// Returns the LNURL-withdraw parameters for wallet display.
pub async fn lnurlw_request(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
    Query(sun_params): Query<SunParams>,
) -> Result<Json<LnurlWithdrawResponse>, (StatusCode, Json<LnurlCallbackResponse>)> {
    tracing::info!("LNURL-withdraw request for location {}", location_id);

    // Verify SUN and get withdrawal info
    let (location, _nfc_card, _counter, withdrawable_msats) =
        match verify_and_prepare_withdrawal(&state, &location_id, &sun_params).await {
            Ok(result) => result,
            Err(response) => {
                let error_msg = response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string());
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(LnurlCallbackResponse::error(error_msg)),
                ));
            }
        };

    // Build callback URL with SUN params preserved
    let callback = format!(
        "{}/api/lnurlw/{}/callback?p={}&c={}",
        state.base_url,
        location_id,
        urlencoding::encode(&sun_params.p),
        urlencoding::encode(&sun_params.c)
    );

    // k1 is a unique identifier for this withdraw request
    // We use a combination of location_id and counter for uniqueness
    let k1 = format!("{}:{}", location_id, sun_params.p);

    Ok(Json(LnurlWithdrawResponse {
        tag: "withdrawRequest".to_string(),
        callback,
        k1,
        default_description: format!("Withdraw from SatsHunt location: {}", location.name),
        min_withdrawable: withdrawable_msats,
        max_withdrawable: withdrawable_msats,
    }))
}

/// Query parameters for LNURL-withdraw callback
#[derive(Debug, Deserialize)]
pub struct LnurlCallbackParams {
    /// NTAG424 encrypted picc_data
    pub p: String,
    /// NTAG424 CMAC signature
    pub c: String,
    /// k1 from initial request (for verification)
    pub k1: String,
    /// BOLT11 invoice from wallet
    pub pr: String,
}

/// LNURL-withdraw callback (LUD-03)
///
/// GET /api/lnurlw/{location_id}/callback?p={}&c={}&k1={}&pr={}
///
/// Called by wallet with the BOLT11 invoice to pay.
pub async fn lnurlw_callback(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
    Query(params): Query<LnurlCallbackParams>,
) -> Result<Json<LnurlCallbackResponse>, (StatusCode, Json<LnurlCallbackResponse>)> {
    tracing::info!("LNURL-withdraw callback for location {}", location_id);

    let sun_params = SunParams {
        p: params.p,
        c: params.c,
    };

    // Verify SUN and get withdrawal info
    let (location, _nfc_card, counter, _withdrawable_msats) =
        match verify_and_prepare_withdrawal(&state, &location_id, &sun_params).await {
            Ok(result) => result,
            Err(response) => {
                let error_msg = response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string());
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(LnurlCallbackResponse::error(error_msg)),
                ));
            }
        };

    // Basic invoice validation
    let invoice = params.pr.trim();
    if !invoice.to_lowercase().starts_with("lnbc") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(LnurlCallbackResponse::error(
                "Invalid invoice format. Must be a valid Lightning invoice.",
            )),
        ));
    }

    // Atomically claim the withdrawal (updates counter and zeros balance)
    // This prevents double-spending even if the same scan is used multiple times
    let claimed_msats = match state
        .db
        .claim_withdrawal(&location_id, counter as i64, &state.balance_config)
        .await
    {
        Ok(Some(msats)) => msats,
        Ok(None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(LnurlCallbackResponse::error(
                    "This scan has already been used. Please scan the sticker again.",
                )),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to claim withdrawal: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LnurlCallbackResponse::error(
                    "Failed to process withdrawal. Please try again.",
                )),
            ));
        }
    };

    let claimed_sats = claimed_msats / 1000;

    // Pay the invoice
    if let Err(e) = state.lightning.pay_invoice(invoice).await {
        tracing::error!("Failed to pay invoice: {}", e);
        // Note: The balance was already claimed, so the user will need to scan again.
        // This is intentional to prevent double-spending attempts.
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(LnurlCallbackResponse::error(
                "Payment failed. Please scan the sticker again to retry.",
            )),
        ));
    }

    // Record the successful withdrawal
    record_withdrawal(&state, &location_id, claimed_msats).await;

    tracing::info!(
        "Successful LNURL-withdraw from {}: {} sats",
        location.name,
        claimed_sats
    );

    Ok(Json(LnurlCallbackResponse::ok()))
}

/// Withdraw via pasted BOLT11 invoice
///
/// POST /api/withdraw/{location_id}/invoice?picc_data={}&cmac={}
/// Body: { "invoice": "lnbc..." }
pub async fn withdraw_invoice(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
    Query(sun_params): Query<SunParams>,
    Json(payload): Json<InvoiceWithdrawRequest>,
) -> Result<Json<WithdrawResponse>, StatusCode> {
    tracing::info!("Invoice withdrawal request for location {}", location_id);

    // Verify SUN and get withdrawal info
    let (location, _nfc_card, counter, _withdrawable_msats) =
        match verify_and_prepare_withdrawal(&state, &location_id, &sun_params).await {
            Ok(result) => result,
            Err(response) => return Ok(Json(response)),
        };

    // Basic invoice validation
    let invoice = payload.invoice.trim();
    if !invoice.to_lowercase().starts_with("lnbc") {
        return Ok(Json(WithdrawResponse::error(
            "Invalid invoice format. Must be a valid Lightning invoice.",
        )));
    }

    // Atomically claim the withdrawal (updates counter and zeros balance)
    // This prevents double-spending even if the same scan is used multiple times
    let claimed_msats = match state
        .db
        .claim_withdrawal(&location_id, counter as i64, &state.balance_config)
        .await
    {
        Ok(Some(msats)) => msats,
        Ok(None) => {
            return Ok(Json(WithdrawResponse::error(
                "This scan has already been used. Please scan the sticker again.",
            )));
        }
        Err(e) => {
            tracing::error!("Failed to claim withdrawal: {}", e);
            return Ok(Json(WithdrawResponse::error(
                "Failed to process withdrawal. Please try again.",
            )));
        }
    };

    let claimed_sats = claimed_msats / 1000;

    // Pay the invoice
    if let Err(e) = state.lightning.pay_invoice(invoice).await {
        tracing::error!("Failed to pay invoice: {}", e);
        // Note: The balance was already claimed, so the user will need to scan again.
        // This is intentional to prevent double-spending attempts.
        return Ok(Json(WithdrawResponse::error(
            "Payment failed. Please scan the sticker again to retry.",
        )));
    }

    // Record the successful withdrawal
    record_withdrawal(&state, &location_id, claimed_msats).await;

    tracing::info!(
        "Successful invoice withdrawal from {}: {} sats",
        location.name,
        claimed_sats
    );

    Ok(Json(WithdrawResponse::success(claimed_sats, &location_id)))
}

// ============================================================================
// Custodial Wallet Collection Endpoint
// ============================================================================

/// Response for collection endpoint
#[derive(Debug, Serialize)]
pub struct CollectResponse {
    pub success: bool,
    pub collected_sats: i64,
    pub new_balance_sats: i64,
    pub location_name: String,
    /// User ID to store in localStorage as backup
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl CollectResponse {
    fn success(
        collected_sats: i64,
        new_balance_sats: i64,
        location_name: String,
        user_id: String,
    ) -> Self {
        Self {
            success: true,
            collected_sats,
            new_balance_sats,
            location_name,
            user_id,
            error: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            collected_sats: 0,
            new_balance_sats: 0,
            location_name: String::new(),
            user_id: String::new(),
            error: Some(message.into()),
        }
    }
}

/// Collect sats from a location into user's custodial balance
///
/// POST /api/collect/{location_id}?p={picc_data}&c={cmac}
///
/// This is the primary endpoint for the custodial wallet system.
/// Users collect sats into their balance instead of receiving immediate Lightning payments.
pub async fn collect_sats(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
    Query(sun_params): Query<SunParams>,
    user: CookieUser,
) -> impl IntoResponse {
    tracing::info!(
        "Collection request for location {} by user {} (kind: {:?})",
        location_id,
        user.user_id,
        user.kind
    );

    // Helper to build error response with cookie jar
    let error_response = |jar: PrivateCookieJar, status: StatusCode, msg: &str| {
        (jar, (status, Json(CollectResponse::error(msg)))).into_response()
    };

    // Verify the SUN message
    let verification =
        match ntag424::verify_sun_message(&state.db, &location_id, &sun_params.p, &sun_params.c)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                let msg = match e {
                    ntag424::SunError::ReplayDetected { .. } => {
                        "This scan has already been used. Please scan the sticker again."
                    }
                    ntag424::SunError::CmacMismatch => {
                        "Invalid NFC scan. Please scan the sticker again."
                    }
                    ntag424::SunError::UidMismatch { .. } => "Invalid NFC card for this location.",
                    ntag424::SunError::CardNotFound | ntag424::SunError::CardNotProgrammed => {
                        "NFC card not configured."
                    }
                    _ => {
                        tracing::error!("SUN verification error: {}", e);
                        "Verification failed. Please try again."
                    }
                };
                return error_response(user.jar, StatusCode::BAD_REQUEST, msg);
            }
        };

    let location = verification.location;
    let counter = verification.counter;

    // Atomically claim collection (creates user if needed, updates counter, computes balance)
    let collected_msats = match state
        .db
        .claim_collection(
            &location_id,
            &user.user_id,
            counter as i64,
            &state.balance_config,
        )
        .await
    {
        Ok(Some(msats)) => msats,
        Ok(None) => {
            tracing::warn!("Collection already claimed or no balance");
            return error_response(
                user.jar,
                StatusCode::CONFLICT,
                "This scan has already been used. Please scan the sticker again.",
            );
        }
        Err(e) => {
            tracing::error!("Collection failed: {}", e);
            return error_response(
                user.jar,
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to process collection. Please try again.",
            );
        }
    };

    let collected_sats = collected_msats / 1000;

    // Get new balance
    let new_balance_msats = state
        .db
        .get_user_balance(&user.user_id)
        .await
        .unwrap_or(collected_msats);
    let new_balance_sats = new_balance_msats / 1000;

    tracing::info!(
        "User {} collected {} sats from {}, new balance: {} sats",
        user.user_id,
        collected_sats,
        location.name,
        new_balance_sats
    );

    // Return response with the cookie jar (handles setting new cookie if needed)
    (
        user.jar,
        Json(CollectResponse::success(
            collected_sats,
            new_balance_sats,
            location.name,
            user.user_id,
        )),
    )
        .into_response()
}

// ============================================================================
// Claim Endpoint (new separate scan/claim flow)
// ============================================================================

/// Claim sats from a previous scan.
///
/// POST /api/claim/{scan_id}
///
/// The user must be the one who made the scan, and within 1 hour.
pub async fn claim_sats(
    State(state): State<Arc<AppState>>,
    Path(scan_id): Path<String>,
    user: CookieUser,
) -> impl IntoResponse {
    tracing::info!(
        "Claim request for scan {} by user {}",
        scan_id,
        user.user_id
    );

    let result = state
        .db
        .claim_from_scan(&scan_id, &user.user_id, &state.balance_config)
        .await;

    match result {
        Ok(ClaimResult::Success { msats, claim_id }) => {
            let collected_sats = msats / 1000;

            // Get new balance
            let new_balance_msats = state
                .db
                .get_user_balance(&user.user_id)
                .await
                .unwrap_or(msats);
            let new_balance_sats = new_balance_msats / 1000;

            tracing::info!(
                "User {} claimed {} sats (claim_id: {}), new balance: {} sats",
                user.user_id,
                collected_sats,
                claim_id,
                new_balance_sats
            );

            (
                user.jar,
                Json(CollectResponse::success(
                    collected_sats,
                    new_balance_sats,
                    "this location".to_string(),
                    user.user_id,
                )),
            )
                .into_response()
        }
        Ok(ClaimResult::ScanNotFound) => (
            user.jar,
            (
                StatusCode::NOT_FOUND,
                Json(CollectResponse::error("Scan not found.")),
            ),
        )
            .into_response(),
        Ok(ClaimResult::NotYourScan) => (
            user.jar,
            (
                StatusCode::FORBIDDEN,
                Json(CollectResponse::error("This scan belongs to someone else.")),
            ),
        )
            .into_response(),
        Ok(ClaimResult::AlreadyClaimed) => (
            user.jar,
            (
                StatusCode::CONFLICT,
                Json(CollectResponse::error(
                    "This scan has already been claimed.",
                )),
            ),
        )
            .into_response(),
        Ok(ClaimResult::Expired) => (
            user.jar,
            (
                StatusCode::GONE,
                Json(CollectResponse::error(
                    "This scan has expired. Please scan again.",
                )),
            ),
        )
            .into_response(),
        Ok(ClaimResult::NotLastScanner) => (
            user.jar,
            (
                StatusCode::CONFLICT,
                Json(CollectResponse::error(
                    "Someone else scanned after you. Please scan again.",
                )),
            ),
        )
            .into_response(),
        Ok(ClaimResult::NoBalance) => (
            user.jar,
            (
                StatusCode::BAD_REQUEST,
                Json(CollectResponse::error(
                    "No sats available at this location.",
                )),
            ),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Claim failed: {}", e);
            (
                user.jar,
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(CollectResponse::error(
                        "Failed to process claim. Please try again.",
                    )),
                ),
            )
                .into_response()
        }
    }
}

// ============================================================================
// Custodial Wallet Withdrawal Endpoint
// ============================================================================

/// Request body for wallet withdrawal
#[derive(Debug, Deserialize)]
pub struct WalletWithdrawRequest {
    pub ln_address: String,
}

/// Response for wallet withdrawal endpoint
#[derive(Debug, Serialize)]
pub struct WalletWithdrawResponse {
    pub success: bool,
    pub withdrawn_sats: i64,
    pub new_balance_sats: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl WalletWithdrawResponse {
    fn success(withdrawn_sats: i64, new_balance_sats: i64) -> Self {
        Self {
            success: true,
            withdrawn_sats,
            new_balance_sats,
            error: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            withdrawn_sats: 0,
            new_balance_sats: 0,
            error: Some(message.into()),
        }
    }
}

/// Withdraw sats from user's custodial balance via Lightning Address
///
/// POST /api/wallet/withdraw
/// Body: { "ln_address": "user@domain.com" }
///
/// Withdraws the user's entire balance to the specified Lightning Address.
pub async fn wallet_withdraw(
    State(state): State<Arc<AppState>>,
    user: CookieUser,
    Json(payload): Json<WalletWithdrawRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "Wallet withdrawal request from user {} to {}",
        user.user_id,
        payload.ln_address
    );

    // Helper to build error response with cookie jar
    let error_response = |jar: PrivateCookieJar, status: StatusCode, msg: &str| {
        (jar, (status, Json(WalletWithdrawResponse::error(msg)))).into_response()
    };

    // Get user balance
    let balance_msats = match state.db.get_user_balance(&user.user_id).await {
        Ok(balance) => balance,
        Err(e) => {
            tracing::error!("Failed to get user balance: {}", e);
            return error_response(
                user.jar,
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get balance. Please try again.",
            );
        }
    };

    // Calculate withdrawable amount after fees
    let (max_withdraw_msats, fee_msats) = calculate_withdrawal_fees(balance_msats);

    // Check minimum withdrawal amount (need enough to cover fees + at least 1 sat)
    if max_withdraw_msats < 1000 {
        return error_response(
            user.jar,
            StatusCode::BAD_REQUEST,
            "Insufficient balance to cover fees. You need at least ~3 sats to withdraw.",
        );
    }

    // Round down to whole sats for the invoice
    let withdraw_sats = max_withdraw_msats / 1000;
    let withdraw_msats = withdraw_sats * 1000;

    // Resolve LN address and get invoice (do this before reserving balance)
    let invoice = match lnurl::get_invoice_for_ln_address(&payload.ln_address, withdraw_msats).await
    {
        Ok(inv) => inv,
        Err(lnurl::LnurlError::InvalidFormat(msg)) => {
            return error_response(
                user.jar,
                StatusCode::BAD_REQUEST,
                &format!("Invalid Lightning address: {}", msg),
            );
        }
        Err(lnurl::LnurlError::AmountOutOfRange { min, max, .. }) => {
            return error_response(
                user.jar,
                StatusCode::BAD_REQUEST,
                &format!(
                    "Amount {} sats is outside the allowed range ({}-{} sats)",
                    withdraw_sats,
                    min / 1000,
                    max / 1000
                ),
            );
        }
        Err(e) => {
            tracing::error!("LN address resolution failed: {}", e);
            return error_response(
                user.jar,
                StatusCode::BAD_REQUEST,
                "Could not resolve Lightning address. Please check and try again.",
            );
        }
    };

    // Create pending withdrawal to reserve the balance (including fees)
    let withdrawal_id = match state
        .db
        .create_pending_withdrawal(&user.user_id, withdraw_msats, fee_msats, &invoice)
        .await
    {
        Ok(Some(id)) => id,
        Ok(None) => {
            return error_response(
                user.jar,
                StatusCode::CONFLICT,
                "Insufficient balance. Please try again.",
            );
        }
        Err(e) => {
            tracing::error!("Failed to create pending withdrawal: {}", e);
            return error_response(
                user.jar,
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to process withdrawal. Please try again.",
            );
        }
    };

    let withdrawn_sats = withdraw_msats / 1000;

    // Pay the invoice
    if let Err(e) = state.lightning.pay_invoice(&invoice).await {
        tracing::error!("Failed to pay invoice: {}", e);
        // Mark withdrawal as failed - balance will be released
        if let Err(e) = state.db.fail_pending_withdrawal(&withdrawal_id).await {
            tracing::error!("Failed to mark withdrawal as failed: {}", e);
        }
        return error_response(
            user.jar,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Payment failed. Please try again.",
        );
    }

    // Mark withdrawal as completed
    if let Err(e) = state.db.complete_pending_withdrawal(&withdrawal_id).await {
        tracing::error!("Failed to complete withdrawal: {}", e);
        // Payment succeeded but we couldn't record it - this is bad but rare
    }

    // Get new balance
    let new_balance_msats = state.db.get_user_balance(&user.user_id).await.unwrap_or(0);
    let new_balance_sats = new_balance_msats / 1000;

    tracing::info!(
        "User {} withdrew {} sats to {}, new balance: {} sats",
        user.user_id,
        withdrawn_sats,
        payload.ln_address,
        new_balance_sats
    );

    (
        user.jar,
        Json(WalletWithdrawResponse::success(
            withdrawn_sats,
            new_balance_sats,
        )),
    )
        .into_response()
}

/// Request body for wallet invoice withdrawal
#[derive(Debug, Deserialize)]
pub struct WalletWithdrawInvoiceRequest {
    pub invoice: String,
}

/// Withdraw sats from user's custodial balance via pasted BOLT11 invoice
///
/// POST /api/wallet/withdraw/invoice
/// Body: { "invoice": "lnbc..." }
///
/// Pays the provided invoice from the user's balance. The invoice amount
/// must be less than or equal to the user's balance.
pub async fn wallet_withdraw_invoice(
    State(state): State<Arc<AppState>>,
    user: CookieUser,
    Json(payload): Json<WalletWithdrawInvoiceRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "Wallet invoice withdrawal request from user {}",
        user.user_id
    );

    // Helper to build error response with cookie jar
    let error_response = |jar: PrivateCookieJar, status: StatusCode, msg: &str| {
        (jar, (status, Json(WalletWithdrawResponse::error(msg)))).into_response()
    };

    // Parse and validate the invoice
    let invoice_str = payload.invoice.trim();
    let invoice: lightning_invoice::Bolt11Invoice = match invoice_str.parse() {
        Ok(inv) => inv,
        Err(e) => {
            tracing::warn!("Invalid invoice format: {}", e);
            return error_response(
                user.jar,
                StatusCode::BAD_REQUEST,
                "Invalid invoice format. Please paste a valid Lightning invoice.",
            );
        }
    };

    // Get invoice amount in msats
    let invoice_msats = match invoice.amount_milli_satoshis() {
        Some(msats) => msats as i64,
        None => {
            return error_response(
                user.jar,
                StatusCode::BAD_REQUEST,
                "Invoice must specify an amount. Please create an invoice with a specific amount.",
            );
        }
    };

    if invoice_msats < 1000 {
        return error_response(
            user.jar,
            StatusCode::BAD_REQUEST,
            "Invoice amount must be at least 1 sat.",
        );
    }

    // Get user balance
    let balance_msats = match state.db.get_user_balance(&user.user_id).await {
        Ok(balance) => balance,
        Err(e) => {
            tracing::error!("Failed to get user balance: {}", e);
            return error_response(
                user.jar,
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get balance. Please try again.",
            );
        }
    };

    // Check if user has enough balance for the invoice amount + fees
    let fee_msats = match check_invoice_with_fees(invoice_msats, balance_msats) {
        Ok(fee) => fee,
        Err(msg) => return error_response(user.jar, StatusCode::BAD_REQUEST, &msg),
    };

    // Create pending withdrawal to reserve the balance (including fees)
    let withdrawal_id = match state
        .db
        .create_pending_withdrawal(&user.user_id, invoice_msats, fee_msats, invoice_str)
        .await
    {
        Ok(Some(id)) => id,
        Ok(None) => {
            return error_response(
                user.jar,
                StatusCode::CONFLICT,
                "Insufficient balance. Please try again.",
            );
        }
        Err(e) => {
            tracing::error!("Failed to create pending withdrawal: {}", e);
            return error_response(
                user.jar,
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to process withdrawal. Please try again.",
            );
        }
    };

    let withdrawn_sats = invoice_msats / 1000;

    // Pay the invoice
    if let Err(e) = state.lightning.pay_invoice(invoice_str).await {
        tracing::error!("Failed to pay invoice: {}", e);
        // Mark withdrawal as failed - balance will be released
        if let Err(e) = state.db.fail_pending_withdrawal(&withdrawal_id).await {
            tracing::error!("Failed to mark withdrawal as failed: {}", e);
        }
        return error_response(
            user.jar,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Payment failed. Please try again.",
        );
    }

    // Mark withdrawal as completed
    if let Err(e) = state.db.complete_pending_withdrawal(&withdrawal_id).await {
        tracing::error!("Failed to complete withdrawal: {}", e);
        // Payment succeeded but we couldn't record it - this is bad but rare
    }

    // Get new balance
    let new_balance_msats = state.db.get_user_balance(&user.user_id).await.unwrap_or(0);
    let new_balance_sats = new_balance_msats / 1000;

    tracing::info!(
        "User {} withdrew {} sats via invoice, new balance: {} sats",
        user.user_id,
        withdrawn_sats,
        new_balance_sats
    );

    (
        user.jar,
        Json(WalletWithdrawResponse::success(
            withdrawn_sats,
            new_balance_sats,
        )),
    )
        .into_response()
}

// ============================================================================
// Wallet LNURL-withdraw Endpoints (LUD-03)
// ============================================================================

/// Query parameters for wallet LNURL-withdraw initial request
#[derive(Debug, Deserialize)]
pub struct WalletLnurlwParams {
    /// Signed withdrawal token (format: "user_id:timestamp:signature")
    pub token: String,
}

/// LNURL-withdraw initial request for wallet balance
///
/// GET /api/wallet/lnurlw?token={signed_token}
///
/// Returns LNURL-withdraw parameters for the user's wallet balance.
/// This allows users to withdraw by scanning a QR code with their Lightning wallet.
/// The token is signed with HMAC to authenticate the user without cookies.
pub async fn wallet_lnurlw_request(
    State(state): State<Arc<AppState>>,
    Query(params): Query<WalletLnurlwParams>,
) -> Result<Json<LnurlWithdrawResponse>, (StatusCode, Json<LnurlCallbackResponse>)> {
    tracing::info!("Wallet LNURL-withdraw request with token");

    // Verify the signed token and extract user_id
    let user_id =
        verify_withdraw_token(&state.withdraw_secret, &params.token).ok_or_else(|| {
            tracing::warn!("Invalid or expired withdraw token");
            (
                StatusCode::UNAUTHORIZED,
                Json(LnurlCallbackResponse::error(
                    "Invalid or expired withdrawal link. Please refresh and try again.",
                )),
            )
        })?;

    tracing::info!("Wallet LNURL-withdraw request from user {}", user_id);

    // Get user balance
    let balance_msats = match state.db.get_user_balance(&user_id).await {
        Ok(balance) => balance,
        Err(e) => {
            tracing::error!("Failed to get user balance: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LnurlCallbackResponse::error("Failed to get balance.")),
            ));
        }
    };

    // Calculate withdrawable amount after fees
    let (max_withdraw_msats, _fee_msats) = calculate_withdrawal_fees(balance_msats);

    // Check minimum withdrawal amount (need enough to cover fees + at least 1 sat)
    if max_withdraw_msats < 1000 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(LnurlCallbackResponse::error(
                "Insufficient balance to cover fees. You need at least ~3 sats to withdraw.",
            )),
        ));
    }

    // Round down to whole sats
    let withdraw_sats = max_withdraw_msats / 1000;
    let withdraw_msats = withdraw_sats * 1000;

    // Build callback URL - pass the token through for the callback to verify
    let callback = format!(
        "{}/api/wallet/lnurlw/callback?token={}",
        state.base_url,
        urlencoding::encode(&params.token)
    );

    // k1 can be anything unique - we use user_id for logging purposes
    let k1 = user_id.clone();

    Ok(Json(LnurlWithdrawResponse {
        tag: "withdrawRequest".to_string(),
        callback,
        k1,
        default_description: format!("Withdraw {} sats from SatsHunt wallet", withdraw_sats),
        min_withdrawable: withdraw_msats,
        max_withdrawable: withdraw_msats,
    }))
}

/// Query parameters for wallet LNURL-withdraw callback
#[derive(Debug, Deserialize)]
pub struct WalletLnurlCallbackParams {
    /// Signed withdrawal token (format: "user_id:timestamp:signature")
    pub token: String,
    /// k1 from initial request (for logging only)
    pub k1: String,
    /// BOLT11 invoice from wallet
    pub pr: String,
}

/// LNURL-withdraw callback for wallet balance
///
/// GET /api/wallet/lnurlw/callback?token={signed_token}&k1={user_id}&pr={invoice}
///
/// Called by the user's Lightning wallet with the invoice to pay.
pub async fn wallet_lnurlw_callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<WalletLnurlCallbackParams>,
) -> Result<Json<LnurlCallbackResponse>, (StatusCode, Json<LnurlCallbackResponse>)> {
    tracing::info!("Wallet LNURL-withdraw callback for k1 {}", params.k1);

    // Verify the signed token and extract user_id
    let user_id =
        verify_withdraw_token(&state.withdraw_secret, &params.token).ok_or_else(|| {
            tracing::warn!("Invalid or expired withdraw token in callback");
            (
                StatusCode::UNAUTHORIZED,
                Json(LnurlCallbackResponse::error(
                    "Invalid or expired withdrawal link. Please refresh and try again.",
                )),
            )
        })?;

    tracing::info!("Wallet LNURL-withdraw callback for user {}", user_id);
    let invoice = params.pr.trim();

    // Parse the invoice to get the amount
    let parsed_invoice: lightning_invoice::Bolt11Invoice = invoice.parse().map_err(|e| {
        tracing::error!("Failed to parse invoice: {}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(LnurlCallbackResponse::error(
                "Invalid invoice format. Must be a valid Lightning invoice.",
            )),
        )
    })?;

    // Get the invoice amount (we don't support amountless invoices)
    let invoice_msats = parsed_invoice.amount_milli_satoshis().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(LnurlCallbackResponse::error(
                "Invoice must specify an amount.",
            )),
        )
    })? as i64;

    // Get user balance (already accounts for pending withdrawals)
    let balance_msats = state.db.get_user_balance(&user_id).await.map_err(|e| {
        tracing::error!("Failed to get user balance: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(LnurlCallbackResponse::error("Failed to get balance.")),
        )
    })?;

    // Check if user has enough balance for invoice + fees
    let fee_msats = check_invoice_with_fees(invoice_msats, balance_msats).map_err(|msg| {
        (
            StatusCode::BAD_REQUEST,
            Json(LnurlCallbackResponse::error(&msg)),
        )
    })?;

    // Create pending withdrawal to reserve the balance (including fees)
    let withdrawal_id = state
        .db
        .create_pending_withdrawal(&user_id, invoice_msats, fee_msats, invoice)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create pending withdrawal: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LnurlCallbackResponse::error(
                    "Failed to process withdrawal. Please try again.",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::CONFLICT,
                Json(LnurlCallbackResponse::error(
                    "Insufficient balance. Please try again.",
                )),
            )
        })?;

    // Pay the invoice
    if let Err(e) = state.lightning.pay_invoice(invoice).await {
        tracing::error!("Failed to pay invoice: {}", e);
        // Mark withdrawal as failed to release the reserved balance
        if let Err(e) = state.db.fail_pending_withdrawal(&withdrawal_id).await {
            tracing::error!("Failed to mark withdrawal as failed: {}", e);
        }
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(LnurlCallbackResponse::error(
                "Payment failed. Please try again.",
            )),
        ));
    }

    // Mark withdrawal as completed
    if let Err(e) = state.db.complete_pending_withdrawal(&withdrawal_id).await {
        tracing::error!("Failed to mark withdrawal as completed: {}", e);
        // Payment succeeded but we couldn't update the status - log but don't fail
    }

    let withdrawn_sats = invoice_msats / 1000;
    tracing::info!(
        "Successful wallet LNURL-withdraw for user {}: {} sats",
        user_id,
        withdrawn_sats
    );

    Ok(Json(LnurlCallbackResponse::ok()))
}

// ============================================================================
// Admin Endpoints
// ============================================================================

/// Request body for updating user role
#[derive(Debug, Deserialize)]
pub struct UpdateUserRoleRequest {
    pub role: String,
}

/// Update a user's role (admin only)
///
/// POST /api/admin/users/{user_id}/role
/// Body: { "role": "user" | "creator" | "admin" }
pub async fn update_user_role(
    State(state): State<Arc<AppState>>,
    auth: RequireRegistered,
    Path(user_id): Path<String>,
    Form(payload): Form<UpdateUserRoleRequest>,
) -> Result<StatusCode, StatusCode> {
    // Require admin role
    auth.ensure_role(UserRole::Admin)
        .map_err(|_| StatusCode::FORBIDDEN)?;

    tracing::info!(
        "Admin {} updating role for user {} to {}",
        auth.user_id,
        user_id,
        payload.role
    );

    // Parse the role
    let new_role: UserRole = payload.role.parse().map_err(|_| {
        tracing::warn!("Invalid role: {}", payload.role);
        StatusCode::BAD_REQUEST
    })?;

    // Update the user's role
    let result = state
        .db
        .update_user_role(&user_id, new_role)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update user role: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        tracing::warn!("User not found: {}", user_id);
        return Err(StatusCode::NOT_FOUND);
    }

    tracing::info!(
        "Admin {} updated user {} role to {}",
        auth.user_id,
        user_id,
        new_role
    );

    Ok(StatusCode::OK)
}

/// Deactivate a location
///
/// POST /api/locations/{location_id}/deactivate
///
/// Creators can deactivate their own active locations.
/// Admins can deactivate any active location (using admin_deactivated status).
pub async fn deactivate_location(
    State(state): State<Arc<AppState>>,
    auth: RequireRegistered,
    Path(location_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!(
        "Deactivate request for location {} by user {}",
        location_id,
        auth.user_id
    );

    // Get the location
    let location = state
        .db
        .get_location(&location_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Location not found: {}", location_id);
            StatusCode::NOT_FOUND
        })?;

    // Check if location is active (only active locations can be deactivated)
    if !location.is_active() {
        tracing::warn!("Location {} is not active", location_id);
        return Err(StatusCode::BAD_REQUEST);
    }

    let is_admin = auth.has_role(UserRole::Admin);
    let is_owner = location.user_id == auth.user_id;

    // Must be owner or admin
    if !is_owner && !is_admin {
        tracing::warn!(
            "User {} attempted to deactivate location {} owned by {}",
            auth.user_id,
            location_id,
            location.user_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Creators must be at least Creator role to deactivate their own locations
    if is_owner && !is_admin {
        auth.ensure_role(UserRole::Creator)
            .map_err(|_| StatusCode::FORBIDDEN)?;
    }

    // Set status based on who is deactivating
    let new_status = if is_admin && !is_owner {
        "admin_deactivated"
    } else {
        "deactivated"
    };

    state
        .db
        .update_location_status(&location_id, new_status)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update location status: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        "Location {} deactivated by {} (status: {})",
        location.name,
        auth.user_id,
        new_status
    );

    Ok(StatusCode::OK)
}

/// Reactivate a location
///
/// POST /api/locations/{location_id}/reactivate
///
/// Creators can reactivate their own deactivated locations (but NOT admin_deactivated).
/// Admins can reactivate any deactivated location (including admin_deactivated).
pub async fn reactivate_location(
    State(state): State<Arc<AppState>>,
    auth: RequireRegistered,
    Path(location_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!(
        "Reactivate request for location {} by user {}",
        location_id,
        auth.user_id
    );

    // Get the location
    let location = state
        .db
        .get_location(&location_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Location not found: {}", location_id);
            StatusCode::NOT_FOUND
        })?;

    // Check if location is deactivated
    if !location.is_deactivated() && !location.is_admin_deactivated() {
        tracing::warn!("Location {} is not deactivated", location_id);
        return Err(StatusCode::BAD_REQUEST);
    }

    let is_admin = auth.has_role(UserRole::Admin);
    let is_owner = location.user_id == auth.user_id;

    // Must be owner or admin
    if !is_owner && !is_admin {
        tracing::warn!(
            "User {} attempted to reactivate location {} owned by {}",
            auth.user_id,
            location_id,
            location.user_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // If admin_deactivated, only admins can reactivate
    if location.is_admin_deactivated() && !is_admin {
        tracing::warn!(
            "User {} attempted to reactivate admin-deactivated location {}",
            auth.user_id,
            location_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Creators must be at least Creator role to reactivate their own locations
    if is_owner && !is_admin {
        auth.ensure_role(UserRole::Creator)
            .map_err(|_| StatusCode::FORBIDDEN)?;
    }

    state
        .db
        .update_location_status(&location_id, "active")
        .await
        .map_err(|e| {
            tracing::error!("Failed to update location status: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!("Location {} reactivated by {}", location.name, auth.user_id);

    Ok(StatusCode::OK)
}
