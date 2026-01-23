use crate::{
    auth::AuthUser,
    db::Database,
    donation::NewDonation,
    lightning::{Lightning, LightningService},
    lnurl, ntag424,
};
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
use tokio::fs;
use tokio::sync::mpsc;

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
    pub max_sats_per_location: i64,
    pub donation_sender: mpsc::UnboundedSender<NewDonation>,
}

pub async fn create_location(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateLocationRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!(
        "Creating location: {} at ({}, {}) with max {} sats",
        payload.name,
        payload.latitude,
        payload.longitude,
        state.max_sats_per_location
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

    tracing::info!("Creating invoice for donation of {} sats", payload.amount);

    // Generate Lightning invoice
    let description = format!("SatsHunt donation: {} sats", payload.amount);
    let invoice = state
        .lightning
        .create_invoice(payload.amount as u64, &description)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create invoice: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let amount_msats = payload.amount * 1000;

    // Store pending donation in database for resilient tracking
    state
        .db
        .create_pending_donation(invoice.clone(), amount_msats)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create pending donation: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Notify donation service to start awaiting payment
    if let Err(e) = state.donation_sender.send(NewDonation {
        invoice: invoice.clone(),
        amount_msats,
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
    // Invoice format: {invoice_string}:{amount}
    let parts: Vec<&str> = invoice_and_amount.split(':').collect();
    if parts.len() != 2 {
        tracing::error!("Invalid invoice format");
        return Err(StatusCode::BAD_REQUEST);
    }

    let invoice = parts[0];
    let amount: i64 = parts[1].parse().map_err(|_| {
        tracing::error!("Invalid amount in path");
        StatusCode::BAD_REQUEST
    })?;

    tracing::info!("Polling for payment of {} sats invoice", amount);

    // Poll the database for up to 5 minutes (300 seconds) with 2-second intervals
    const MAX_POLLS: u32 = 150;
    const POLL_INTERVAL_MS: u64 = 2000;

    for poll in 0..MAX_POLLS {
        // Check if the pending donation is completed
        match state.db.get_pending_donation_by_invoice(invoice).await {
            Ok(Some(donation)) if donation.is_completed() => {
                tracing::info!("Payment confirmed for {} sats donation", amount);

                // Get current pool total
                let pool = state.db.get_donation_pool().await.map_err(|e| {
                    tracing::error!("Failed to get donation pool: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

                // Return success HTML fragment for HTMX to swap in
                let html = format!(
                    r#"<div id="paymentStatus" class="bg-green-900 border border-green-700 text-green-200 px-4 py-3 rounded-lg">
                        <p class="font-semibold">✓ Payment received!</p>
                        <p class="text-sm mt-1">Thank you for donating {} sats!</p>
                    </div>
                    <div class="text-center mt-4">
                        <p class="text-sm text-slate-400 mb-1">New Pool Total</p>
                        <p class="text-4xl font-bold text-yellow-400">{} ⚡</p>
                    </div>"#,
                    amount,
                    pool.total_sats()
                );

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
    let html = r#"<div id="paymentStatus" class="bg-yellow-900 border border-yellow-700 text-yellow-200 px-4 py-3 rounded-lg">
            <p class="font-semibold">⏳ Still waiting for payment...</p>
            <p class="text-sm mt-1">The invoice is still valid. If you've already paid, your donation will be credited shortly.</p>
            <p class="text-sm mt-1">You can safely close this page - the payment will be processed automatically.</p>
        </div>"#;

    Ok(axum::response::Html(html.to_string()))
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

    // Get location by write token
    let location = state
        .db
        .get_location_by_write_token(&write_token)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Invalid or used write token: {}", write_token);
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

        if existing_card.is_some() {
            // Card already exists - handle based on onExisting parameter
            match on_existing {
                Some("UpdateVersion") => {
                    tracing::info!("Updating version for existing card");
                    state
                        .db
                        .increment_nfc_card_version(&location.id)
                        .await
                        .map_err(|e| {
                            tracing::error!("Failed to increment version: {}", e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;

                    // Update UID and mark as programmed
                    state
                        .db
                        .update_nfc_card_uid_and_mark_programmed(&location.id, uid)
                        .await
                        .map_err(|e| {
                            tracing::error!("Failed to update UID: {}", e);
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
                    tracing::info!("Card already exists, keeping version");
                    // Just update the UID
                    state
                        .db
                        .update_nfc_card_uid_and_mark_programmed(&location.id, uid)
                        .await
                        .map_err(|e| {
                            tracing::error!("Failed to update UID: {}", e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;
                }
            }
        } else {
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

            // Update UID and mark as programmed
            state
                .db
                .update_nfc_card_uid_and_mark_programmed(&location.id, uid)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update UID: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            existing_card = Some(card);
        }

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

    // If location has msats, return them to donation pool
    if location.current_msats > 0 {
        state
            .db
            .add_to_donation_pool(location.current_msats)
            .await
            .map_err(|e| {
                tracing::error!("Failed to return msats to donation pool: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        tracing::info!(
            "Returned {} sats to donation pool from deleted location",
            location.current_sats()
        );
    }

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

/// Calculate slowdown factor based on how full the location is
/// Formula: slowdown = 1 / (1 + exp(k * (fill_ratio - 0.8)))
/// As location fills up past 80%, refill rate slows down
fn calculate_slowdown_factor(current_msats: i64, max_msats: i64) -> f64 {
    const K: f64 = 0.1; // steepness parameter
    const THRESHOLD: f64 = 0.8; // start slowing down at 80% full

    let fill_ratio = current_msats as f64 / max_msats as f64;
    let exponent = K * (fill_ratio - THRESHOLD);
    1.0 / (1.0 + exponent.exp())
}

/// Manually trigger the refill process for all locations
/// Uses formula: refill_per_location = (pool * 0.00016) / num_locations per minute
/// With slowdown as location fills up
pub async fn manual_refill(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!("Manual refill triggered");

    let locations = state.db.list_active_locations().await.map_err(|e| {
        tracing::error!("Failed to list active locations: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let num_locations = locations.len();
    if num_locations == 0 {
        return Ok(Json(json!({
            "success": true,
            "locations_refilled": 0,
            "total_sats_refilled": 0,
            "message": "No active locations to refill"
        })));
    }

    let donation_pool = state.db.get_donation_pool().await.map_err(|e| {
        tracing::error!("Failed to get donation pool: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let now = Utc::now();
    let mut total_refilled_msats = 0i64;
    let mut locations_refilled = 0;

    // Calculate base refill rate per location per minute based on pool size
    // Formula: (pool * 0.016%) / num_locations
    const POOL_PERCENTAGE_PER_MINUTE: f64 = 0.00016;
    let base_msats_per_location_per_minute =
        ((donation_pool.total_msats as f64 * POOL_PERCENTAGE_PER_MINUTE) / num_locations as f64)
            .round() as i64;

    tracing::info!(
        "Base refill rate: {} sats per location per minute (pool: {} sats, locations: {})",
        base_msats_per_location_per_minute / 1000,
        donation_pool.total_msats / 1000,
        num_locations
    );

    for location in locations {
        // Calculate how much time has passed since last activity (refill or withdraw)
        // We use the smaller delta (more recent activity) to avoid gaming
        let minutes_since_activity = (now - location.last_activity_at()).num_minutes();

        if minutes_since_activity < 1 {
            continue; // Not time to refill yet
        }

        let max_msats = state.max_sats_per_location * 1000;

        // Apply slowdown factor based on how full the location is
        let slowdown_factor = calculate_slowdown_factor(location.current_msats, max_msats);
        let adjusted_rate_msats =
            (base_msats_per_location_per_minute as f64 * slowdown_factor).round() as i64;

        // Calculate refill amount based on minutes elapsed and adjusted rate
        let refill_amount_msats = minutes_since_activity * adjusted_rate_msats;
        let new_balance_msats = (location.current_msats + refill_amount_msats).min(max_msats);
        let actual_refill_msats = new_balance_msats - location.current_msats;

        if actual_refill_msats <= 0 {
            continue; // Already at max
        }

        let balance_before = location.current_msats;

        // Update location balance
        state
            .db
            .update_location_msats(&location.id, new_balance_msats)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update location msats: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        state
            .db
            .update_last_refill(&location.id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update last refill: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Record the refill in the log
        state
            .db
            .record_refill(
                &location.id,
                actual_refill_msats,
                balance_before,
                new_balance_msats,
                base_msats_per_location_per_minute,
                slowdown_factor,
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to record refill: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        total_refilled_msats += actual_refill_msats;
        locations_refilled += 1;

        tracing::info!(
            "Refilled location {} with {} sats (now at {}/{}, rate: {} sats/min, slowdown: {:.2}x)",
            location.name,
            actual_refill_msats / 1000,
            new_balance_msats / 1000,
            state.max_sats_per_location,
            adjusted_rate_msats / 1000,
            slowdown_factor
        );
    }

    // Subtract from donation pool
    if total_refilled_msats > 0 {
        state
            .db
            .subtract_from_donation_pool(total_refilled_msats)
            .await
            .map_err(|e| {
                tracing::error!("Failed to subtract from donation pool: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    let new_pool = state.db.get_donation_pool().await.map_err(|e| {
        tracing::error!("Failed to get donation pool: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "success": true,
        "locations_refilled": locations_refilled,
        "total_sats_refilled": total_refilled_msats / 1000,
        "pool_before": donation_pool.total_sats(),
        "pool_after": new_pool.total_sats()
    })))
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

    // Check if location has sats available
    let withdrawable_msats = location.withdrawable_msats();
    if withdrawable_msats <= 0 {
        return Err(WithdrawResponse::error(
            "No sats available at this location.",
        ));
    }

    Ok((location, nfc_card, counter, withdrawable_msats))
}

/// Record a successful withdrawal scan (called after payment succeeds)
async fn record_withdrawal(state: &AppState, location_id: &str, amount_msats: i64) {
    // Record the scan - this is best-effort, payment already succeeded
    if let Err(e) = state.db.record_scan(location_id, amount_msats).await {
        tracing::error!("Failed to record scan: {}", e);
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
        .claim_withdrawal(&location_id, counter as i64)
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
        .claim_withdrawal(&location_id, counter as i64)
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
        .claim_withdrawal(&location_id, counter as i64)
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
