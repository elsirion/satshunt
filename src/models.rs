use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethod {
    Password {
        password_hash: String,
    },
    OAuthGoogle {
        google_id: String,
    },
    OAuthGithub {
        github_id: String,
    },
    /// Anonymous users identified by signed cookie UUID
    Anonymous {},
}

impl AuthMethod {
    pub fn to_type_string(&self) -> &'static str {
        match self {
            AuthMethod::Password { .. } => "password",
            AuthMethod::OAuthGoogle { .. } => "oauth_google",
            AuthMethod::OAuthGithub { .. } => "oauth_github",
            AuthMethod::Anonymous {} => "anonymous",
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
            "anonymous" => Ok(AuthMethod::Anonymous {}),
            _ => Err(anyhow::anyhow!("Unknown auth method: {}", type_str)),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    /// Username - None for anonymous users
    pub username: Option<String>,
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

    /// Check if this is an anonymous user
    pub fn is_anonymous(&self) -> bool {
        self.auth_method == "anonymous"
    }

    /// Get display name - username for registered users, truncated ID for anonymous
    pub fn display_name(&self) -> String {
        self.username
            .clone()
            .unwrap_or_else(|| format!("anon_{}", &self.id[..8]))
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

    /// Get the withdrawable amount in msats (same as current balance since withdrawals are internal)
    pub fn withdrawable_msats(&self) -> i64 {
        self.current_msats
    }

    /// Get the withdrawable amount in sats for display
    pub fn withdrawable_sats(&self) -> i64 {
        self.current_sats()
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Photo {
    pub id: String,
    pub location_id: String,
    pub file_path: String,
    pub uploaded_at: DateTime<Utc>,
}

/// Status of a donation in the payment lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DonationStatus {
    /// Invoice has been created, waiting for payment
    Created,
    /// Payment has been received
    Received,
    /// Invoice timed out without payment
    TimedOut,
}

impl DonationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Received => "received",
            Self::TimedOut => "timed_out",
        }
    }
}

impl std::fmt::Display for DonationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for DonationStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "created" => Ok(Self::Created),
            "received" => Ok(Self::Received),
            "timed_out" => Ok(Self::TimedOut),
            _ => Err(anyhow::anyhow!("Invalid donation status: {}", s)),
        }
    }
}

impl TryFrom<String> for DonationStatus {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

/// A donation to the platform (global or location-specific)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Donation {
    pub id: String,
    /// Location ID for location-specific donations (None = global donation)
    pub location_id: Option<String>,
    pub invoice: String,
    pub amount_msats: i64,
    #[sqlx(try_from = "String")]
    pub status: DonationStatus,
    pub created_at: DateTime<Utc>,
    pub received_at: Option<DateTime<Utc>>,
}

impl Donation {
    pub fn is_created(&self) -> bool {
        self.status == DonationStatus::Created
    }

    pub fn is_received(&self) -> bool {
        self.status == DonationStatus::Received
    }

    pub fn is_timed_out(&self) -> bool {
        self.status == DonationStatus::TimedOut
    }

    /// Get amount in sats for display
    pub fn amount_sats(&self) -> i64 {
        self.amount_msats / 1000
    }
}

/// Debit from a location's donation pool (when refills use the pool)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LocationPoolDebit {
    pub id: String,
    pub location_id: String,
    pub amount_msats: i64,
    pub created_at: DateTime<Utc>,
}

impl LocationPoolDebit {
    /// Get amount in sats for display
    pub fn amount_sats(&self) -> i64 {
        self.amount_msats / 1000
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Scan {
    pub id: String,
    pub location_id: String,
    pub msats_withdrawn: i64,
    pub scanned_at: DateTime<Utc>,
    /// User who collected sats from this scan (None for legacy scans before custodial system)
    pub user_id: Option<String>,
}

impl Scan {
    /// Get withdrawn amount in sats for display
    pub fn sats_withdrawn(&self) -> i64 {
        self.msats_withdrawn / 1000
    }
}

/// User transaction for tracking sat collections and withdrawals in the custodial wallet
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserTransaction {
    pub id: String,
    pub user_id: String,
    /// Location where sats were collected from (None for withdrawals)
    pub location_id: Option<String>,
    pub msats: i64,
    /// Transaction type: 'collect' or 'withdraw'
    pub transaction_type: String,
    pub created_at: DateTime<Utc>,
}

impl UserTransaction {
    /// Get amount in sats for display
    pub fn sats(&self) -> i64 {
        self.msats / 1000
    }

    pub fn is_collect(&self) -> bool {
        self.transaction_type == "collect"
    }

    pub fn is_withdraw(&self) -> bool {
        self.transaction_type == "withdraw"
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

/// Status of a pending withdrawal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WithdrawalStatus {
    Pending,
    Completed,
    Failed,
}

impl WithdrawalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

impl std::fmt::Display for WithdrawalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for WithdrawalStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(anyhow::anyhow!("Invalid withdrawal status: {}", s)),
        }
    }
}

impl TryFrom<String> for WithdrawalStatus {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

/// Pending withdrawal from custodial wallet.
/// Used to prevent double-spending by reserving balance before payment attempt.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PendingWithdrawal {
    pub id: String,
    pub user_id: String,
    pub msats: i64,
    pub invoice: String,
    #[sqlx(try_from = "String")]
    pub status: WithdrawalStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl PendingWithdrawal {
    pub fn is_pending(&self) -> bool {
        self.status == WithdrawalStatus::Pending
    }

    pub fn is_completed(&self) -> bool {
        self.status == WithdrawalStatus::Completed
    }

    pub fn is_failed(&self) -> bool {
        self.status == WithdrawalStatus::Failed
    }

    /// Get amount in sats for display
    pub fn sats(&self) -> i64 {
        self.msats / 1000
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
    fn test_withdrawable_msats_equals_current() {
        // Withdrawable amount equals current balance (no fees for internal transactions)
        let location = make_test_location(1000);
        assert_eq!(location.withdrawable_msats(), 1000);
    }

    #[test]
    fn test_withdrawable_msats_normal() {
        let location = make_test_location(10000);
        assert_eq!(location.withdrawable_msats(), 10000);
    }

    #[test]
    fn test_withdrawable_msats_large() {
        let location = make_test_location(1000000);
        assert_eq!(location.withdrawable_msats(), 1000000);
    }

    #[test]
    fn test_withdrawable_sats() {
        let location = make_test_location(10000);
        // 10000 msats = 10 sats
        assert_eq!(location.withdrawable_sats(), 10);
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
    fn test_auth_method_anonymous_roundtrip() {
        let auth = AuthMethod::Anonymous {};

        let json = auth.to_json().unwrap();
        let parsed = AuthMethod::from_json("anonymous", &json).unwrap();

        match parsed {
            AuthMethod::Anonymous {} => {}
            _ => panic!("Expected Anonymous variant"),
        }

        assert_eq!(auth.to_type_string(), "anonymous");
    }

    #[test]
    fn test_auth_method_from_json_unknown_type() {
        let result = AuthMethod::from_json("unknown", "{}");
        assert!(result.is_err());
    }

    #[test]
    fn test_user_display_name() {
        let now = Utc::now();

        // Registered user with username
        let registered_user = User {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            username: Some("testuser".to_string()),
            email: None,
            auth_method: "password".to_string(),
            auth_data: "{}".to_string(),
            created_at: now,
            last_login_at: None,
        };
        assert_eq!(registered_user.display_name(), "testuser");
        assert!(!registered_user.is_anonymous());

        // Anonymous user without username
        let anon_user = User {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            username: None,
            email: None,
            auth_method: "anonymous".to_string(),
            auth_data: "{}".to_string(),
            created_at: now,
            last_login_at: None,
        };
        assert_eq!(anon_user.display_name(), "anon_550e8400");
        assert!(anon_user.is_anonymous());
    }

    #[test]
    fn test_user_transaction() {
        let now = Utc::now();

        let collect_tx = UserTransaction {
            id: "tx-1".to_string(),
            user_id: "user-1".to_string(),
            location_id: Some("loc-1".to_string()),
            msats: 5000,
            transaction_type: "collect".to_string(),
            created_at: now,
        };
        assert!(collect_tx.is_collect());
        assert!(!collect_tx.is_withdraw());
        assert_eq!(collect_tx.sats(), 5);

        let withdraw_tx = UserTransaction {
            id: "tx-2".to_string(),
            user_id: "user-1".to_string(),
            location_id: None,
            msats: 3000,
            transaction_type: "withdraw".to_string(),
            created_at: now,
        };
        assert!(!withdraw_tx.is_collect());
        assert!(withdraw_tx.is_withdraw());
        assert_eq!(withdraw_tx.sats(), 3);
    }

    #[test]
    fn test_donation_amount_sats() {
        let donation = Donation {
            id: "test-id".to_string(),
            location_id: None,
            invoice: "lnbc...".to_string(),
            amount_msats: 123456,
            status: DonationStatus::Received,
            created_at: Utc::now(),
            received_at: Some(Utc::now()),
        };
        assert_eq!(donation.amount_sats(), 123);
        assert!(donation.is_received());
    }

    #[test]
    fn test_scan_sats_withdrawn() {
        let scan = Scan {
            id: "scan-id".to_string(),
            location_id: "loc-id".to_string(),
            msats_withdrawn: 5678,
            scanned_at: Utc::now(),
            user_id: None,
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
