use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Location {
    pub id: String,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub current_sats: i64,
    pub max_sats: i64,
    pub lnurlw_secret: String,
    pub last_refill_at: DateTime<Utc>,
    pub write_token: Option<String>,
    pub write_token_used: bool,
    pub write_token_created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Photo {
    pub id: String,
    pub location_id: String,
    pub file_path: String,
    pub uploaded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DonationPool {
    pub id: i64,
    pub total_sats: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Scan {
    pub id: String,
    pub location_id: String,
    pub sats_withdrawn: i64,
    pub scanned_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CreateLocationRequest {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub description: Option<String>,
    pub max_sats: i64,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct LocationWithPhotos {
    #[serde(flatten)]
    pub location: Location,
    pub photos: Vec<Photo>,
}

#[derive(Debug, Serialize)]
pub struct Stats {
    pub total_locations: i64,
    pub total_sats_available: i64,
    pub total_scans: i64,
    pub donation_pool_sats: i64,
}
