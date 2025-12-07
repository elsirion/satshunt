use crate::{
    auth::AuthUser,
    db::Database,
    lightning::{LightningService, LnurlCallbackResponse, LnurlWithdrawCallback, LnurlWithdrawResponse},
};
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
use tokio::{fs, io::AsyncWriteExt};
use chrono::Utc;

pub struct AppState {
    pub db: Database,
    pub lightning: LightningService,
    pub upload_dir: PathBuf,
    pub base_url: String,
    pub max_sats_per_location: i64,
}

pub async fn create_location(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!("Received location creation request");

    let mut name = None;
    let mut latitude = None;
    let mut longitude = None;
    let mut description = None;
    let mut photo_files = Vec::new();

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::error!("Failed to read multipart field: {}", e);
        StatusCode::BAD_REQUEST
    })? {
        let field_name = field.name().unwrap_or("").to_string();
        tracing::debug!("Processing field: {}", field_name);

        match field_name.as_str() {
            "name" => {
                name = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "latitude" => {
                let text = field.text().await.map_err(|e| {
                    tracing::error!("Failed to read latitude field: {}", e);
                    StatusCode::BAD_REQUEST
                })?;
                latitude = Some(text.parse::<f64>().map_err(|e| {
                    tracing::error!("Failed to parse latitude '{}': {}", text, e);
                    StatusCode::BAD_REQUEST
                })?);
            }
            "longitude" => {
                let text = field.text().await.map_err(|e| {
                    tracing::error!("Failed to read longitude field: {}", e);
                    StatusCode::BAD_REQUEST
                })?;
                longitude = Some(text.parse::<f64>().map_err(|e| {
                    tracing::error!("Failed to parse longitude '{}': {}", text, e);
                    StatusCode::BAD_REQUEST
                })?);
            }
            "description" => {
                let text = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                if !text.is_empty() {
                    description = Some(text);
                }
            }
            "photos" => {
                if let Some(filename) = field.file_name() {
                    let filename = filename.to_string();
                    let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                    photo_files.push((filename, data));
                }
            }
            _ => {}
        }
    }

    // Log what we received
    tracing::info!(
        "Parsed form data - name: {:?}, lat: {:?}, lng: {:?}, desc: {:?}, photos: {}",
        name, latitude, longitude, description, photo_files.len()
    );

    // Validate required fields
    let name = name.ok_or_else(|| {
        tracing::error!("Missing required field: name");
        StatusCode::BAD_REQUEST
    })?;
    let latitude = latitude.ok_or_else(|| {
        tracing::error!("Missing required field: latitude");
        StatusCode::BAD_REQUEST
    })?;
    let longitude = longitude.ok_or_else(|| {
        tracing::error!("Missing required field: longitude");
        StatusCode::BAD_REQUEST
    })?;

    tracing::info!("Creating location: {} at ({}, {}) with max {} sats", name, latitude, longitude, state.max_sats_per_location);

    // Generate LNURL secret
    let lnurlw_secret = LightningService::generate_lnurlw_secret();

    // Create location in database
    let location = state
        .db
        .create_location(name, latitude, longitude, description, lnurlw_secret, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Give the location initial 5 sats from donation pool for activation
    const INITIAL_SATS: i64 = 5;

    // Check if donation pool has enough
    let donation_pool = state.db.get_donation_pool().await.map_err(|e| {
        tracing::error!("Failed to get donation pool: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if donation_pool.total_sats >= INITIAL_SATS {
        // Deduct from donation pool
        state.db.subtract_from_donation_pool(INITIAL_SATS).await.map_err(|e| {
            tracing::error!("Failed to subtract from donation pool: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        // Add to location
        state.db.update_location_sats(&location.id, INITIAL_SATS).await.map_err(|e| {
            tracing::error!("Failed to update location sats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        tracing::info!("Gave {} initial sats to new location: {}", INITIAL_SATS, location.name);
    } else {
        tracing::warn!(
            "Donation pool too low ({} sats) to give initial {} sats to location: {}",
            donation_pool.total_sats,
            INITIAL_SATS,
            location.name
        );
    }

    // Save uploaded photos
    for (filename, data) in photo_files {
        let unique_filename = format!("{}_{}", uuid::Uuid::new_v4(), filename);
        let file_path = state.upload_dir.join(&unique_filename);

        let mut file = fs::File::create(&file_path).await.map_err(|e| {
            tracing::error!("Failed to create file: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        file.write_all(&data).await.map_err(|e| {
            tracing::error!("Failed to write file: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        state
            .db
            .add_photo(&location.id, unique_filename)
            .await
            .map_err(|e| {
                tracing::error!("Failed to save photo record: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    Ok(Json(json!({
        "location_id": location.id,
        "write_token": location.write_token
    })))
}

/// LNURL-withdraw endpoint
/// Returns the withdrawal offer when scanned
pub async fn lnurlw_endpoint(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
) -> Result<Json<LnurlWithdrawResponse>, StatusCode> {
    let location = state
        .db
        .get_location(&location_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let callback_url = format!("{}/api/lnurlw/{}/callback", state.base_url, location_id);

    let response = LnurlWithdrawResponse::new(
        callback_url,
        location.lnurlw_secret.clone(),
        location.current_sats,
        &location.name,
    );

    Ok(Json(response))
}

/// LNURL-withdraw callback
/// Processes the actual withdrawal when user provides their invoice
pub async fn lnurlw_callback(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
    Query(params): Query<LnurlWithdrawCallback>,
) -> Result<Json<LnurlCallbackResponse>, StatusCode> {
    let location = state
        .db
        .get_location(&location_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Verify secret
    if params.secret != location.lnurlw_secret {
        return Ok(Json(LnurlCallbackResponse::error("Invalid secret")));
    }

    // Check if location has sats available
    if location.current_sats <= 0 {
        return Ok(Json(LnurlCallbackResponse::error("No sats available")));
    }

    // TODO: Parse invoice to get the amount
    // For now, we'll withdraw all available sats
    let amount_to_withdraw = location.current_sats;

    // Pay the invoice
    state
        .lightning
        .pay_invoice(&params.pr)
        .await
        .map_err(|e| {
            tracing::error!("Failed to pay invoice: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Update location balance
    let new_balance = location.current_sats - amount_to_withdraw;
    state
        .db
        .update_location_sats(&location_id, new_balance)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update location sats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Record the scan
    state
        .db
        .record_scan(&location_id, amount_to_withdraw)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record scan: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Activate location on first successful scan if it's not already active
    if !location.is_active() {
        state
            .db
            .update_location_status(&location_id, "active")
            .await
            .map_err(|e| {
                tracing::error!("Failed to activate location: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Mark write token as used now that location is activated
        if let Some(token) = &location.write_token {
            state
                .db
                .mark_write_token_used(token)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to mark write token as used: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }

        tracing::info!("Location {} activated on first successful scan", location.name);
    }

    tracing::info!(
        "Withdrawal from location {} for {} sats",
        location.name,
        amount_to_withdraw
    );

    Ok(Json(LnurlCallbackResponse::ok()))
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, StatusCode> {
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

    // Generate QR code
    use qrcode::QrCode;
    use image::Luma;

    let qr_code = QrCode::new(&invoice).map_err(|e| {
        tracing::error!("Failed to create QR code: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let qr_image = qr_code.render::<Luma<u8>>().build();

    // Convert to PNG bytes
    let mut png_bytes = Vec::new();
    use image::codecs::png::PngEncoder;
    use image::{ImageEncoder, ExtendedColorType};

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

    tracing::info!("Invoice created successfully");

    Ok(Json(json!({
        "invoice": invoice,
        "qr_code": format!("data:image/png;base64,{}", qr_base64),
        "amount": payload.amount
    })))
}

/// Wait for invoice payment and update donation pool
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

    tracing::info!("Waiting for payment of {} sats invoice", amount);

    // Wait for payment (this blocks until paid)
    state.lightning.await_payment(invoice).await.map_err(|e| {
        tracing::error!("Failed to await payment: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Payment received! Adding {} sats to donation pool", amount);

    // Add to donation pool
    let pool = state.db.add_to_donation_pool(amount).await.map_err(|e| {
        tracing::error!("Failed to add to donation pool: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Donation pool updated. New total: {} sats", pool.total_sats);

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
        amount, pool.total_sats
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

    let lnurlw_url = format!("{}/api/lnurlw/{}", state.base_url.replace("https://", "lnurlw://").replace("http://", "lnurlw://"), location.id);


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

        tracing::info!("Location {} marked as programmed (write token still valid for retries)", location.name);
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

/// Manually trigger the refill process for all locations
pub async fn manual_refill(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!("Manual refill triggered");

    let locations = state.db.list_active_locations().await.map_err(|e| {
        tracing::error!("Failed to list active locations: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let donation_pool = state.db.get_donation_pool().await.map_err(|e| {
        tracing::error!("Failed to get donation pool: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let now = Utc::now();
    let mut total_refilled = 0i64;
    let mut locations_refilled = 0;
    let mut remaining_pool = donation_pool.total_sats;

    for location in locations {
        // Calculate how much time has passed since last refill in minutes
        let minutes_since_refill = (now - location.last_refill_at).num_minutes();

        if minutes_since_refill < 1 {
            continue; // Not time to refill yet
        }

        // Calculate refill amount (1 sat per minute)
        let refill_amount = minutes_since_refill;
        let new_balance = (location.current_sats + refill_amount).min(state.max_sats_per_location);
        let actual_refill = new_balance - location.current_sats;

        if actual_refill <= 0 {
            continue; // Already at max
        }

        // Check if remaining pool has enough
        if remaining_pool < actual_refill {
            tracing::warn!(
                "Donation pool too low to refill location {}: need {}, have {}",
                location.name,
                actual_refill,
                remaining_pool
            );
            continue;
        }

        // Update location balance
        state.db.update_location_sats(&location.id, new_balance).await.map_err(|e| {
            tracing::error!("Failed to update location sats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        state.db.update_last_refill(&location.id).await.map_err(|e| {
            tracing::error!("Failed to update last refill: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        total_refilled += actual_refill;
        remaining_pool -= actual_refill;
        locations_refilled += 1;

        tracing::info!(
            "Refilled location {} with {} sats (now at {}/{})",
            location.name,
            actual_refill,
            new_balance,
            state.max_sats_per_location
        );
    }

    // Subtract from donation pool
    if total_refilled > 0 {
        state.db.subtract_from_donation_pool(total_refilled).await.map_err(|e| {
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
        "total_sats_refilled": total_refilled,
        "pool_before": donation_pool.total_sats,
        "pool_after": new_pool.total_sats
    })))
}
