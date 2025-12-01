use crate::{
    db::Database,
    lightning::{LightningService, LnurlCallbackResponse, LnurlWithdrawCallback, LnurlWithdrawResponse},
};
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
use tokio::{fs, io::AsyncWriteExt};

pub struct AppState {
    pub db: Database,
    pub lightning: LightningService,
    pub upload_dir: PathBuf,
    pub base_url: String,
}

pub async fn create_location(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!("Received location creation request");

    let mut name = None;
    let mut latitude = None;
    let mut longitude = None;
    let mut description = None;
    let mut max_sats = None;
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
            "max_sats" => {
                let text = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                max_sats = Some(text.parse::<i64>().map_err(|_| StatusCode::BAD_REQUEST)?);
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
        "Parsed form data - name: {:?}, lat: {:?}, lng: {:?}, desc: {:?}, max_sats: {:?}, photos: {}",
        name, latitude, longitude, description, max_sats, photo_files.len()
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
    let max_sats = max_sats.ok_or_else(|| {
        tracing::error!("Missing required field: max_sats");
        StatusCode::BAD_REQUEST
    })?;

    tracing::info!("Creating location: {} at ({}, {}) with {} sats", name, latitude, longitude, max_sats);

    // Generate LNURL secret
    let lnurlw_secret = LightningService::generate_lnurlw_secret();

    // Create location in database
    let location = state
        .db
        .create_location(name, latitude, longitude, description, max_sats, lnurlw_secret)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create location: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

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
