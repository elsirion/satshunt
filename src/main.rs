mod config;
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
use clap::Parser;
use config::Config;
use handlers::api::AppState;
use std::sync::Arc;
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

    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Parse configuration from CLI args and environment variables
    let config = Config::parse();

    // Get derived paths
    let base_url = config.get_base_url();
    let database_url = config.get_database_url();
    let uploads_dir = config.get_uploads_dir();
    let blitzi_dir = config.get_blitzi_dir();

    // Ensure directories exist
    tokio::fs::create_dir_all(&config.data_dir).await?;
    tokio::fs::create_dir_all(&uploads_dir).await?;
    tokio::fs::create_dir_all(&blitzi_dir).await?;
    tracing::info!("📁 Data directory: {}", config.data_dir.display());
    tracing::info!("📁 Uploads directory: {}", uploads_dir.display());
    tracing::info!("📁 Blitzi directory: {}", blitzi_dir.display());

    // Initialize database (this will also create the database file)
    let db = Arc::new(db::Database::new(&database_url).await?);
    tracing::info!("💾 Database initialized: {}", database_url);

    // Initialize Lightning service
    let lightning = lightning::LightningService::new(&blitzi_dir).await?;
    tracing::info!("Lightning service initialized");

    // Create app state
    let app_state = Arc::new(AppState {
        db: (*db).clone(),
        lightning,
        upload_dir: uploads_dir.clone(),
        base_url: base_url.clone(),
        max_sats_per_location: config.max_sats_per_location,
    });

    // Start refill service
    let refill_service = Arc::new(refill::RefillService::new(
        db.clone(),
        refill::RefillConfig {
            sats_per_hour: config.refill_rate_sats_per_hour,
            check_interval_secs: config.refill_check_interval_secs,
            max_sats_per_location: config.max_sats_per_location,
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
        .route("/donate", get(handlers::donate_page))
        // API routes
        .route("/api/locations", post(handlers::create_location))
        .route("/api/lnurlw/:location_id", get(handlers::lnurlw_endpoint))
        .route(
            "/api/lnurlw/:location_id/callback",
            get(handlers::lnurlw_callback),
        )
        .route("/api/stats", get(handlers::get_stats))
        .route("/api/donate/invoice", post(handlers::create_donation_invoice))
        .route("/api/donate/wait/:invoice_and_amount", get(handlers::wait_for_donation))
        .route("/api/refill/trigger", post(handlers::manual_refill))
        // Static files
        .nest_service("/uploads", ServeDir::new(&uploads_dir))
        // State
        .with_state(app_state)
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("🚀 SatShunt server listening on http://{}", addr);
    tracing::info!("📍 Base URL: {}", base_url);
    tracing::info!("⚙️  Refill rate: {} sats/hour", config.refill_rate_sats_per_hour);
    tracing::info!("⚙️  Max sats per location: {}", config.max_sats_per_location);

    axum::serve(listener, app).await?;

    Ok(())
}
