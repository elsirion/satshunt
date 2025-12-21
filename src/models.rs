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
    pub last_withdraw_at: Option<DateTime<Utc>>,
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

    /// Get the most recent activity time (max of last_refill_at and last_withdraw_at).
    /// Used for calculating refill delta - we use the smaller delta (more recent activity).
    pub fn last_activity_at(&self) -> DateTime<Utc> {
        self.last_withdraw_at
            .map(|withdraw_at| self.last_refill_at.max(withdraw_at))
            .unwrap_or(self.last_refill_at)
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_test_location(current_msats: i64) -> Location {
        let now = Utc::now();
        Location {
            id: "test-id".to_string(),
            name: "Test Location".to_string(),
            latitude: 0.0,
            longitude: 0.0,
            description: None,
            created_at: now,
            current_msats,
            lnurlw_secret: "secret".to_string(),
            last_refill_at: now,
            last_withdraw_at: None,
            write_token: None,
            write_token_used: false,
            write_token_created_at: None,
            user_id: "user-id".to_string(),
            status: "active".to_string(),
        }
    }

    #[test]
    fn test_withdrawable_msats_zero() {
        let location = make_test_location(0);
        assert_eq!(location.withdrawable_msats(), 0);
    }

    #[test]
    fn test_withdrawable_msats_below_fixed_fee() {
        // Fixed fee is 2000 msats (2 sats)
        // With 1000 msats, after 0.5% routing fee (5 msats) and 2000 fixed fee,
        // we should get 0 (can't go negative)
        let location = make_test_location(1000);
        assert_eq!(location.withdrawable_msats(), 0);
    }

    #[test]
    fn test_withdrawable_msats_at_threshold() {
        // At exactly 2000 msats (2 sats):
        // Routing fee: 2000 * 0.005 = 10 msats (ceiled)
        // Fixed fee: 2000 msats
        // Total fees: 2010 msats
        // Withdrawable: 2000 - 2010 = -10 -> 0 (clamped)
        let location = make_test_location(2000);
        assert_eq!(location.withdrawable_msats(), 0);
    }

    #[test]
    fn test_withdrawable_msats_normal() {
        // With 10000 msats (10 sats):
        // Routing fee: 10000 * 0.005 = 50 msats
        // Fixed fee: 2000 msats
        // Total fees: 2050 msats
        // Withdrawable: 10000 - 2050 = 7950 msats
        let location = make_test_location(10000);
        assert_eq!(location.withdrawable_msats(), 7950);
    }

    #[test]
    fn test_withdrawable_msats_large() {
        // With 1000000 msats (1000 sats):
        // Routing fee: 1000000 * 0.005 = 5000 msats
        // Fixed fee: 2000 msats
        // Total fees: 7000 msats
        // Withdrawable: 1000000 - 7000 = 993000 msats
        let location = make_test_location(1000000);
        assert_eq!(location.withdrawable_msats(), 993000);
    }

    #[test]
    fn test_withdrawable_sats() {
        let location = make_test_location(10000);
        // 7950 msats = 7 sats (integer division)
        assert_eq!(location.withdrawable_sats(), 7);
    }

    #[test]
    fn test_current_sats() {
        let location = make_test_location(12345);
        assert_eq!(location.current_sats(), 12);
    }

    #[test]
    fn test_last_activity_at_no_withdraw() {
        let now = Utc::now();
        let mut location = make_test_location(1000);
        location.last_refill_at = now;
        location.last_withdraw_at = None;

        assert_eq!(location.last_activity_at(), now);
    }

    #[test]
    fn test_last_activity_at_withdraw_more_recent() {
        let now = Utc::now();
        let earlier = now - Duration::hours(1);

        let mut location = make_test_location(1000);
        location.last_refill_at = earlier;
        location.last_withdraw_at = Some(now);

        assert_eq!(location.last_activity_at(), now);
    }

    #[test]
    fn test_last_activity_at_refill_more_recent() {
        let now = Utc::now();
        let earlier = now - Duration::hours(1);

        let mut location = make_test_location(1000);
        location.last_refill_at = now;
        location.last_withdraw_at = Some(earlier);

        assert_eq!(location.last_activity_at(), now);
    }

    #[test]
    fn test_location_status_helpers() {
        let mut location = make_test_location(1000);

        location.status = "created".to_string();
        assert!(location.is_created());
        assert!(!location.is_programmed());
        assert!(!location.is_active());

        location.status = "programmed".to_string();
        assert!(!location.is_created());
        assert!(location.is_programmed());
        assert!(!location.is_active());

        location.status = "active".to_string();
        assert!(!location.is_created());
        assert!(!location.is_programmed());
        assert!(location.is_active());
    }

    #[test]
    fn test_auth_method_password_roundtrip() {
        let auth = AuthMethod::Password {
            password_hash: "argon2hash123".to_string(),
        };

        let json = auth.to_json().unwrap();
        let parsed = AuthMethod::from_json("password", &json).unwrap();

        match parsed {
            AuthMethod::Password { password_hash } => {
                assert_eq!(password_hash, "argon2hash123");
            }
            _ => panic!("Expected Password variant"),
        }

        assert_eq!(auth.to_type_string(), "password");
    }

    #[test]
    fn test_auth_method_oauth_google_roundtrip() {
        let auth = AuthMethod::OAuthGoogle {
            google_id: "google123".to_string(),
        };

        let json = auth.to_json().unwrap();
        let parsed = AuthMethod::from_json("oauth_google", &json).unwrap();

        match parsed {
            AuthMethod::OAuthGoogle { google_id } => {
                assert_eq!(google_id, "google123");
            }
            _ => panic!("Expected OAuthGoogle variant"),
        }

        assert_eq!(auth.to_type_string(), "oauth_google");
    }

    #[test]
    fn test_auth_method_oauth_github_roundtrip() {
        let auth = AuthMethod::OAuthGithub {
            github_id: "github456".to_string(),
        };

        let json = auth.to_json().unwrap();
        let parsed = AuthMethod::from_json("oauth_github", &json).unwrap();

        match parsed {
            AuthMethod::OAuthGithub { github_id } => {
                assert_eq!(github_id, "github456");
            }
            _ => panic!("Expected OAuthGithub variant"),
        }

        assert_eq!(auth.to_type_string(), "oauth_github");
    }

    #[test]
    fn test_auth_method_from_json_unknown_type() {
        let result = AuthMethod::from_json("unknown", "{}");
        assert!(result.is_err());
    }

    #[test]
    fn test_donation_pool_total_sats() {
        let pool = DonationPool {
            id: 1,
            total_msats: 123456,
            updated_at: Utc::now(),
        };
        assert_eq!(pool.total_sats(), 123);
    }

    #[test]
    fn test_scan_sats_withdrawn() {
        let scan = Scan {
            id: "scan-id".to_string(),
            location_id: "loc-id".to_string(),
            msats_withdrawn: 5678,
            scanned_at: Utc::now(),
        };
        assert_eq!(scan.sats_withdrawn(), 5);
    }

    #[test]
    fn test_refill_display_methods() {
        let refill = Refill {
            id: "refill-id".to_string(),
            location_id: "loc-id".to_string(),
            msats_added: 1500,
            balance_before_msats: 5000,
            balance_after_msats: 6500,
            base_rate_msats_per_min: 100,
            slowdown_factor: 0.95,
            refilled_at: Utc::now(),
        };

        assert!((refill.sats_added() - 1.5).abs() < 0.001);
        assert!((refill.balance_before_sats() - 5.0).abs() < 0.001);
        assert!((refill.balance_after_sats() - 6.5).abs() < 0.001);
        assert!((refill.base_rate_sats_per_min() - 0.1).abs() < 0.001);
    }
}
