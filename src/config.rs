use clap::Parser;
use std::path::PathBuf;

/// SatsHunt - A Lightning Network treasure hunt service
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// Host address to bind to
    #[arg(long, env = "SH_HOST", default_value = "127.0.0.1")]
    pub host: String,

    /// Port to listen on
    #[arg(long, env = "SH_PORT", default_value = "3000")]
    pub port: String,

    /// Data directory for database, uploads, and Blitzi data
    #[arg(long, env = "SH_DATA_DIR", default_value = "./data")]
    pub data_dir: PathBuf,

    /// Base URL for the application
    #[arg(long, env = "SH_BASE_URL")]
    pub base_url: Option<String>,

    /// Percentage of donation pool to distribute per minute (default: 0.016%)
    /// This is divided equally among all active locations
    #[arg(long, env = "SH_POOL_PERCENTAGE_PER_MINUTE", default_value = "0.00016")]
    pub pool_percentage_per_minute: f64,

    /// Maximum sats per location (global cap)
    #[arg(long, env = "SH_MAX_SATS_PER_LOCATION", default_value = "1000")]
    pub max_sats_per_location: i64,

    /// Refill check interval in seconds
    #[arg(long, env = "SH_REFILL_CHECK_INTERVAL_SECS", default_value = "300")]
    pub refill_check_interval_secs: u64,

    /// Static files directory
    #[arg(long, env = "SH_STATIC_DIR", default_value = "./static")]
    pub static_dir: PathBuf,
}

impl Config {
    /// Get the base URL, defaulting to http://host:port if not set
    pub fn get_base_url(&self) -> String {
        self.base_url
            .clone()
            .unwrap_or_else(|| format!("http://{}:{}", self.host, self.port))
    }

    /// Get the database URL
    pub fn get_database_url(&self) -> String {
        let db_path = self.data_dir.join("satshunt.db");
        format!("sqlite:{}", db_path.display())
    }

    /// Get the uploads directory
    pub fn get_uploads_dir(&self) -> PathBuf {
        self.data_dir.join("uploads")
    }

    /// Get the Blitzi data directory
    pub fn get_blitzi_dir(&self) -> PathBuf {
        self.data_dir.join("blitzi")
    }
}
