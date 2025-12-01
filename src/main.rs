mod db;
mod handlers;
mod lightning;
mod models;
mod refill;
mod templates;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use handlers::api::AppState;
use std::{path::PathBuf, sync::Arc};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "satshunt=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Configuration
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:satshunt.db".to_string());
    let upload_dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());
    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| format!("http://{}:{}", host, port));
    let refill_rate = std::env::var("REFILL_RATE_SATS_PER_HOUR")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    // Ensure upload directory exists
    let upload_path = PathBuf::from(&upload_dir);
    tokio::fs::create_dir_all(&upload_path).await?;

    // Initialize database
    let db = Arc::new(db::Database::new(&database_url).await?);
    tracing::info!("Database initialized");

    // Initialize Lightning service
    let lightning = lightning::LightningService::new()?;
    tracing::info!("Lightning service initialized");

    // Create app state
    let app_state = Arc::new(AppState {
        db: (*db).clone(),
        lightning,
        upload_dir: upload_path.clone(),
        base_url: base_url.clone(),
    });

    // Start refill service
    let refill_service = Arc::new(refill::RefillService::new(
        db.clone(),
        refill::RefillConfig {
            sats_per_hour: refill_rate,
            check_interval_secs: 300, // 5 minutes
        },
    ));

    tokio::spawn(async move {
        refill_service.start().await;
    });

    tracing::info!("Refill service started");

    // Build router
    let app = Router::new()
        // Page routes
        .route("/", get(handlers::home_page))
        .route("/map", get(handlers::map_page))
        .route("/locations/new", get(handlers::new_location_page))
        .route("/locations/:id", get(handlers::location_detail_page))
        .route("/setup/:write_token", get(handlers::nfc_setup_page))
        // API routes
        .route("/api/locations", post(handlers::create_location))
        .route("/api/lnurlw/:location_id", get(handlers::lnurlw_endpoint))
        .route(
            "/api/lnurlw/:location_id/callback",
            get(handlers::lnurlw_callback),
        )
        .route("/api/stats", get(handlers::get_stats))
        // Static files
        .nest_service("/uploads", ServeDir::new(upload_path))
        // State
        .with_state(app_state)
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("🚀 SatShunt server listening on http://{}", addr);
    tracing::info!("📍 Base URL: {}", base_url);

    axum::serve(listener, app).await?;

    Ok(())
}
