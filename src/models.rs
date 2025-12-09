use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethod {
    Password { password_hash: String },
    OAuthGoogle { google_id: String },
    OAuthGithub { github_id: String },
    // Future auth methods can be added here
}

impl AuthMethod {
    pub fn to_type_string(&self) -> &'static str {
        match self {
            AuthMethod::Password { .. } => "password",
            AuthMethod::OAuthGoogle { .. } => "oauth_google",
            AuthMethod::OAuthGithub { .. } => "oauth_github",
        }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    pub fn from_json(type_str: &str, json: &str) -> anyhow::Result<Self> {
        match type_str {
            "password" => {
                let data: serde_json::Value = serde_json::from_str(json)?;
                Ok(AuthMethod::Password {
                    password_hash: data["password_hash"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing password_hash"))?
                        .to_string(),
                })
            }
            "oauth_google" => {
                let data: serde_json::Value = serde_json::from_str(json)?;
                Ok(AuthMethod::OAuthGoogle {
                    google_id: data["google_id"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing google_id"))?
                        .to_string(),
                })
            }
            "oauth_github" => {
                let data: serde_json::Value = serde_json::from_str(json)?;
                Ok(AuthMethod::OAuthGithub {
                    github_id: data["github_id"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing github_id"))?
                        .to_string(),
                })
            }
            _ => Err(anyhow::anyhow!("Unknown auth method: {}", type_str)),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub auth_method: String,
    pub auth_data: String,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

impl User {
    pub fn get_auth_method(&self) -> anyhow::Result<AuthMethod> {
        AuthMethod::from_json(&self.auth_method, &self.auth_data)
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Location {
    pub id: String,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub current_msats: i64,
    pub lnurlw_secret: String,
    pub last_refill_at: DateTime<Utc>,
    pub write_token: Option<String>,
    pub write_token_used: bool,
    pub write_token_created_at: Option<DateTime<Utc>>,
    pub user_id: String,
    pub status: String, // 'created', 'programmed', 'active'
}

impl Location {
    pub fn is_created(&self) -> bool {
        self.status == "created"
    }

    pub fn is_programmed(&self) -> bool {
        self.status == "programmed"
    }

    pub fn is_active(&self) -> bool {
        self.status == "active"
    }

    /// Convert msats to sats for display purposes
    pub fn current_sats(&self) -> i64 {
        self.current_msats / 1000
    }

    /// Calculate the withdrawable amount accounting for fees
    /// Subtracts 2 sats fixed fee and 0.5% routing fee
    pub fn withdrawable_msats(&self) -> i64 {
        // Calculate routing fee (0.5%)
        let routing_fee_msats = (self.current_msats as f64 * 0.005).ceil() as i64;

        // Fixed fee of 2 sats (2000 msats)
        let fixed_fee_msats = 2000;

        // Total fees
        let total_fee_msats = routing_fee_msats + fixed_fee_msats;

        // Withdrawable amount (can't go below 0)
        (self.current_msats - total_fee_msats).max(0)
    }

    /// Get the withdrawable amount in sats for display
    pub fn withdrawable_sats(&self) -> i64 {
        self.withdrawable_msats() / 1000
    }
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
    pub total_msats: i64,
    pub updated_at: DateTime<Utc>,
}

impl DonationPool {
    /// Get total in sats for display
    pub fn total_sats(&self) -> i64 {
        self.total_msats / 1000
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Scan {
    pub id: String,
    pub location_id: String,
    pub msats_withdrawn: i64,
    pub scanned_at: DateTime<Utc>,
}

impl Scan {
    /// Get withdrawn amount in sats for display
    pub fn sats_withdrawn(&self) -> i64 {
        self.msats_withdrawn / 1000
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CreateLocationRequest {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub description: Option<String>,
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

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NfcCard {
    pub id: String,
    pub location_id: String,
    pub k0_auth_key: String,
    pub k1_decrypt_key: String,
    pub k2_cmac_key: String,
    pub k3: String,
    pub k4: String,
    pub uid: Option<String>,
    pub counter: i64,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub programmed_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Refill {
    pub id: String,
    pub location_id: String,
    pub msats_added: i64,
    pub balance_before_msats: i64,
    pub balance_after_msats: i64,
    pub base_rate_msats_per_min: i64,
    pub slowdown_factor: f64,
    pub refilled_at: DateTime<Utc>,
}

impl Refill {
    /// Get amount added in sats for display (with 3 decimal places for msat precision)
    pub fn sats_added(&self) -> f64 {
        self.msats_added as f64 / 1000.0
    }

    /// Get balance before in sats for display (with 3 decimal places for msat precision)
    pub fn balance_before_sats(&self) -> f64 {
        self.balance_before_msats as f64 / 1000.0
    }

    /// Get balance after in sats for display (with 3 decimal places for msat precision)
    pub fn balance_after_sats(&self) -> f64 {
        self.balance_after_msats as f64 / 1000.0
    }

    /// Get base rate in sats per minute for display (with 3 decimal places for msat precision)
    pub fn base_rate_sats_per_min(&self) -> f64 {
        self.base_rate_msats_per_min as f64 / 1000.0
    }
}
