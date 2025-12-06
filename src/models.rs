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
    pub current_sats: i64,
    pub lnurlw_secret: String,
    pub last_refill_at: DateTime<Utc>,
    pub write_token: Option<String>,
    pub write_token_used: bool,
    pub write_token_created_at: Option<DateTime<Utc>>,
    pub user_id: String,
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
