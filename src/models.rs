use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// User roles for access control
/// Roles are hierarchical: Admin > Creator > User
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// Basic user - can collect sats and use wallet
    #[default]
    User,
    /// Creator - can create and manage locations
    Creator,
    /// Admin - full access, can manage users and roles
    Admin,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Creator => "creator",
            Self::Admin => "admin",
        }
    }

    /// Check if this role has at least the given level
    pub fn has_at_least(&self, required: UserRole) -> bool {
        *self >= required
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for UserRole {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(Self::User),
            "creator" => Ok(Self::Creator),
            "admin" => Ok(Self::Admin),
            _ => Err(anyhow::anyhow!("Invalid user role: {}", s)),
        }
    }
}

impl TryFrom<String> for UserRole {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

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
    /// User role for access control
    #[sqlx(try_from = "String")]
    pub role: UserRole,
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

    /// Check if the user has at least the given role level
    pub fn has_role(&self, required: UserRole) -> bool {
        self.role.has_at_least(required)
    }

    /// Check if the user is an admin
    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }

    /// Check if the user is a creator or higher
    pub fn is_creator(&self) -> bool {
        self.role.has_at_least(UserRole::Creator)
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

    pub fn is_deactivated(&self) -> bool {
        self.status == "deactivated"
    }

    pub fn is_admin_deactivated(&self) -> bool {
        self.status == "admin_deactivated"
    }

    /// Check if this location is visible to regular users (active and not deactivated)
    pub fn is_visible(&self) -> bool {
        self.is_active()
    }

    /// Check if this location can be reactivated by its creator
    /// Returns false if admin-deactivated (only admin can reactivate)
    pub fn can_creator_reactivate(&self) -> bool {
        self.is_deactivated()
    }

    // Note: last_activity_at(), current_sats(), withdrawable_msats(), withdrawable_sats() removed
    // Balance is now computed on-demand via balance::compute_balance_msats()
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

/// A validated NFC scan (tap), recorded before claiming
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NfcScan {
    pub id: String,
    pub location_id: String,
    pub user_id: String,
    pub counter: i64,
    pub scanned_at: DateTime<Utc>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub claim_id: Option<String>,
}

impl NfcScan {
    /// Check if this scan is still claimable (within 1 hour, not yet claimed)
    pub fn is_claimable(&self) -> bool {
        if self.claimed_at.is_some() {
            return false;
        }
        let age = Utc::now().signed_duration_since(self.scanned_at);
        age.num_hours() < 1
    }

    /// Check if this scan has expired
    pub fn is_expired(&self) -> bool {
        let age = Utc::now().signed_duration_since(self.scanned_at);
        age.num_hours() >= 1
    }
}

/// A scan record with user display information for the location detail page
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ScanWithUser {
    pub id: String,
    pub location_id: String,
    pub user_id: String,
    pub scanned_at: DateTime<Utc>,
    pub claimed_at: Option<DateTime<Utc>>,
    /// User's username (None for anonymous users)
    pub username: Option<String>,
    /// Amount claimed in msats (None if not claimed)
    pub msats_claimed: Option<i64>,
    /// Whether this is the most recent scan for the location
    pub is_latest: bool,
}

impl ScanWithUser {
    /// Get display name for the scanner
    pub fn scanner_display_name(&self) -> String {
        self.username
            .clone()
            .unwrap_or_else(|| format!("anon_{}", &self.user_id[..8.min(self.user_id.len())]))
    }

    /// Check if this scan has been claimed
    pub fn is_claimed(&self) -> bool {
        self.claimed_at.is_some()
    }

    /// Check if this scan is still claimable (latest, not claimed, within 1 hour)
    pub fn is_claimable(&self) -> bool {
        if self.claimed_at.is_some() || !self.is_latest {
            return false;
        }
        let age = Utc::now().signed_duration_since(self.scanned_at);
        age.num_hours() < 1
    }

    /// Get claimed amount in sats (0 if not claimed)
    pub fn sats_claimed(&self) -> i64 {
        self.msats_claimed.unwrap_or(0) / 1000
    }
}

/// A claim record (sats actually credited to user)
/// Renamed from the old Scan struct
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub location_id: String,
    pub msats_claimed: i64,
    pub claimed_at: DateTime<Utc>,
    /// User who collected sats from this claim (None for legacy claims before custodial system)
    pub user_id: Option<String>,
}

impl Claim {
    /// Get claimed amount in sats for display
    pub fn sats_claimed(&self) -> i64 {
        self.msats_claimed / 1000
    }
}

/// Result of attempting to claim sats from a scan
#[derive(Debug)]
pub enum ClaimResult {
    /// Successfully claimed sats
    Success { msats: i64, claim_id: String },
    /// Scan not found
    ScanNotFound,
    /// User is not the one who made this scan
    NotYourScan,
    /// Scan was already claimed
    AlreadyClaimed,
    /// Scan has expired (>1 hour)
    Expired,
    /// Someone else scanned after this user
    NotLastScanner,
    /// No balance available to claim
    NoBalance,
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

// Note: Refill struct removed - balance is now computed on-demand from donations - scans

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

    // Note: tests for withdrawable_msats, withdrawable_sats, current_sats, last_activity_at removed
    // Balance is now computed on-demand via compute_balance_msats()

    #[test]
    fn test_location_status_helpers() {
        let mut location = make_test_location(1000);

        location.status = "created".to_string();
        assert!(location.is_created());
        assert!(!location.is_programmed());
        assert!(!location.is_active());
        assert!(!location.is_deactivated());
        assert!(!location.is_admin_deactivated());
        assert!(!location.is_visible());

        location.status = "programmed".to_string();
        assert!(!location.is_created());
        assert!(location.is_programmed());
        assert!(!location.is_active());
        assert!(!location.is_deactivated());
        assert!(!location.is_admin_deactivated());
        assert!(!location.is_visible());

        location.status = "active".to_string();
        assert!(!location.is_created());
        assert!(!location.is_programmed());
        assert!(location.is_active());
        assert!(!location.is_deactivated());
        assert!(!location.is_admin_deactivated());
        assert!(location.is_visible());

        location.status = "deactivated".to_string();
        assert!(!location.is_created());
        assert!(!location.is_programmed());
        assert!(!location.is_active());
        assert!(location.is_deactivated());
        assert!(!location.is_admin_deactivated());
        assert!(!location.is_visible());
        assert!(location.can_creator_reactivate());

        location.status = "admin_deactivated".to_string();
        assert!(!location.is_created());
        assert!(!location.is_programmed());
        assert!(!location.is_active());
        assert!(!location.is_deactivated());
        assert!(location.is_admin_deactivated());
        assert!(!location.is_visible());
        assert!(!location.can_creator_reactivate());
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
            role: UserRole::User,
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
            role: UserRole::User,
        };
        assert_eq!(anon_user.display_name(), "anon_550e8400");
        assert!(anon_user.is_anonymous());
    }

    #[test]
    fn test_user_roles() {
        // Test role hierarchy
        assert!(UserRole::Admin.has_at_least(UserRole::Admin));
        assert!(UserRole::Admin.has_at_least(UserRole::Creator));
        assert!(UserRole::Admin.has_at_least(UserRole::User));

        assert!(!UserRole::Creator.has_at_least(UserRole::Admin));
        assert!(UserRole::Creator.has_at_least(UserRole::Creator));
        assert!(UserRole::Creator.has_at_least(UserRole::User));

        assert!(!UserRole::User.has_at_least(UserRole::Admin));
        assert!(!UserRole::User.has_at_least(UserRole::Creator));
        assert!(UserRole::User.has_at_least(UserRole::User));

        // Test user methods
        let now = Utc::now();
        let admin_user = User {
            id: "admin-id".to_string(),
            username: Some("admin".to_string()),
            email: None,
            auth_method: "password".to_string(),
            auth_data: "{}".to_string(),
            created_at: now,
            last_login_at: None,
            role: UserRole::Admin,
        };
        assert!(admin_user.is_admin());
        assert!(admin_user.is_creator());
        assert!(admin_user.has_role(UserRole::User));

        let creator_user = User {
            id: "creator-id".to_string(),
            username: Some("creator".to_string()),
            email: None,
            auth_method: "password".to_string(),
            auth_data: "{}".to_string(),
            created_at: now,
            last_login_at: None,
            role: UserRole::Creator,
        };
        assert!(!creator_user.is_admin());
        assert!(creator_user.is_creator());
        assert!(creator_user.has_role(UserRole::User));
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
    fn test_claim_sats_claimed() {
        let claim = Claim {
            id: "claim-id".to_string(),
            location_id: "loc-id".to_string(),
            msats_claimed: 5678,
            claimed_at: Utc::now(),
            user_id: None,
        };
        assert_eq!(claim.sats_claimed(), 5);
    }

    // Note: test_refill_display_methods removed - Refill struct removed
}
