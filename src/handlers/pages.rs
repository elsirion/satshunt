use crate::{handlers::api::AppState, templates};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};
use std::sync::Arc;

pub async fn home_page(State(state): State<Arc<AppState>>) -> Result<Html<String>, StatusCode> {
    let stats = state.db.get_stats().await.map_err(|e| {
        tracing::error!("Failed to get stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let content = templates::home(&stats);
    let page = templates::base("Home", content);

    Ok(Html(page.into_string()))
}

pub async fn map_page(State(state): State<Arc<AppState>>) -> Result<Html<String>, StatusCode> {
    let locations = state.db.list_locations().await.map_err(|e| {
        tracing::error!("Failed to get locations: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let content = templates::map(&locations);
    let page = templates::base("Map", content);

    Ok(Html(page.into_string()))
}

pub async fn new_location_page() -> Html<String> {
    let content = templates::new_location();
    let page = templates::base("Add Location", content);

    Html(page.into_string())
}

pub async fn location_detail_page(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
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

    let content = templates::location_detail(&location, &photos, &state.base_url);
    let page = templates::base(&location.name, content);

    Ok(Html(page.into_string()))
}

pub async fn nfc_setup_page(
    State(state): State<Arc<AppState>>,
    Path(write_token): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let location = state.db
        .get_location_by_write_token(&write_token)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get location by write token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let content = templates::nfc_setup(&location, &write_token, &state.base_url);
    let page = templates::base("NFC Setup", content);

    Ok(Html(page.into_string()))
}

pub async fn donate_page(State(state): State<Arc<AppState>>) -> Result<Html<String>, StatusCode> {
    let pool = state.db.get_donation_pool().await.map_err(|e| {
        tracing::error!("Failed to get donation pool: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let content = templates::donate(&pool);
    let page = templates::base("Donate", content);

    Ok(Html(page.into_string()))
}
