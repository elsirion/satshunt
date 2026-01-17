use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
};
use clap::Parser;
use config::Config;
use handlers::api::AppState;
use satshunt::{config, db, donation, handlers, lightning, refill};
use std::sync::Arc;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;
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
    let lightning: Arc<dyn lightning::Lightning> =
        Arc::new(lightning::LightningService::new(&blitzi_dir).await?);
    tracing::info!("Lightning service initialized");

    // Start donation service for resilient donation tracking
    let donation_service = Arc::new(donation::DonationService::new(
        db.clone(),
        lightning.clone(),
    ));
    let donation_sender = donation_service.get_sender();

    tokio::spawn({
        let donation_service = donation_service.clone();
        async move {
            donation_service.start().await;
        }
    });

    tracing::info!("Donation service started");

    // Create app state
    let app_state = Arc::new(AppState {
        db: (*db).clone(),
        lightning,
        upload_dir: uploads_dir.clone(),
        base_url: base_url.clone(),
        max_sats_per_location: config.max_sats_per_location,
        donation_sender,
    });

    // Start refill service
    let refill_service = Arc::new(refill::RefillService::new(
        db.clone(),
        refill::RefillConfig {
            pool_percentage_per_minute: config.pool_percentage_per_minute,
            check_interval_secs: config.refill_check_interval_secs,
            max_sats_per_location: config.max_sats_per_location,
        },
    ));

    tokio::spawn(async move {
        refill_service.start().await;
    });

    tracing::info!("Refill service started");

    // Set up session store
    let session_store = SqliteStore::new(db.pool().clone());
    session_store.migrate().await?;

    let session_layer = SessionManagerLayer::new(session_store);

    // Build router
    let app = Router::new()
        // Page routes
        .route("/", get(handlers::home_page))
        .route("/map", get(handlers::map_page))
        .route("/locations/new", get(handlers::new_location_page))
        .route("/locations/:id", get(handlers::location_detail_page))
        .route("/setup/:write_token", get(handlers::nfc_setup_page))
        .route("/donate", get(handlers::donate_page))
        .route("/withdraw/:location_id", get(handlers::withdraw_page))
        .route("/login", get(handlers::login_page).post(handlers::login))
        .route(
            "/register",
            get(handlers::register_page).post(handlers::register),
        )
        .route("/logout", post(handlers::logout))
        .route("/profile", get(handlers::profile_page))
        // API routes
        .route("/api/locations", post(handlers::create_location))
        .route(
            "/api/locations/:location_id/photos",
            post(handlers::upload_photo).layer(DefaultBodyLimit::max(20 * 1024 * 1024)), // 20MB limit for photos
        )
        .route("/api/photos/:photo_id", delete(handlers::delete_photo))
        .route("/api/stats", get(handlers::get_stats))
        .route(
            "/api/donate/invoice",
            post(handlers::create_donation_invoice),
        )
        .route(
            "/api/donate/wait/:invoice_and_amount",
            get(handlers::wait_for_donation),
        )
        .route("/api/refill/trigger", post(handlers::manual_refill))
        // Withdrawal API endpoints
        .route(
            "/api/withdraw/:location_id/ln-address",
            post(handlers::withdraw_ln_address),
        )
        .route(
            "/api/withdraw/:location_id/invoice",
            post(handlers::withdraw_invoice),
        )
        // Boltcard NFC programming endpoint
        .route("/api/boltcard/:write_token", post(handlers::boltcard_keys))
        // Delete location endpoint (non-active only)
        .route(
            "/api/locations/:location_id",
            delete(handlers::delete_location),
        )
        // Static files
        .nest_service("/uploads", ServeDir::new(&uploads_dir))
        .nest_service("/static", ServeDir::new(&config.static_dir))
        // State and middleware
        .with_state(app_state)
        .layer(session_layer)
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("🚀 SatsHunt server listening on http://{}", addr);
    tracing::info!("📍 Base URL: {}", base_url);
    tracing::info!(
        "⚙️  Refill formula: {}% of pool per minute divided by active locations",
        config.pool_percentage_per_minute * 100.0
    );
    tracing::info!(
        "⚙️  Max sats per location: {}",
        config.max_sats_per_location
    );

    axum::serve(listener, app).await?;

    Ok(())
}
