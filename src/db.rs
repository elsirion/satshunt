use crate::models::{
    AuthMethod, Donation, Location, LocationPoolDebit, NfcCard, Photo, Refill, Scan, Stats, User,
    UserRole, UserTransaction, WithdrawalStatus,
};
use anyhow::Result;
use chrono::Utc;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteQueryResult},
    SqlitePool,
};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        // Configure SQLite to create the database file if it doesn't exist
        let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);

        // Connect to the database
        let pool = SqlitePool::connect_with(options).await?;

        // Run migrations to set up the schema
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    #[allow(dead_code)]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // User operations
    pub async fn create_user(
        &self,
        username: String,
        email: Option<String>,
        auth_method: AuthMethod,
    ) -> Result<User> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let method_type = auth_method.to_type_string();
        let method_data = auth_method.to_json()?;

        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, email, auth_method, auth_data, created_at, role)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&username)
        .bind(&email)
        .bind(method_type)
        .bind(&method_data)
        .bind(now)
        .bind(UserRole::User.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    /// Update a user's role (admin only operation)
    pub async fn update_user_role(
        &self,
        user_id: &str,
        role: UserRole,
    ) -> Result<SqliteQueryResult> {
        sqlx::query("UPDATE users SET role = ? WHERE id = ?")
            .bind(role.as_str())
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(Into::into)
    }

    /// List all users (admin only operation)
    pub async fn list_users(&self) -> Result<Vec<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    /// Get user by ID - currently unused but will be needed for user profile pages
    /// and displaying location owner information
    #[allow(dead_code)]
    pub async fn get_user_by_id(&self, id: &str) -> Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn update_last_login(&self, user_id: &str) -> Result<SqliteQueryResult> {
        sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(Into::into)
    }

    // Location operations
    pub async fn create_location(
        &self,
        name: String,
        latitude: f64,
        longitude: f64,
        description: Option<String>,
        lnurlw_secret: String,
        user_id: String,
    ) -> Result<Location> {
        let id = Uuid::new_v4().to_string();
        let write_token = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query_as::<_, Location>(
            r#"
            INSERT INTO locations (
                id, name, latitude, longitude, description,
                current_msats, lnurlw_secret,
                created_at, last_refill_at, write_token, write_token_created_at, user_id, status
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&name)
        .bind(latitude)
        .bind(longitude)
        .bind(&description)
        .bind(0) // current_msats starts at 0
        .bind(&lnurlw_secret)
        .bind(now)
        .bind(now)
        .bind(&write_token)
        .bind(now)
        .bind(&user_id)
        .bind("created") // status starts as 'created'
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_location(&self, id: &str) -> Result<Option<Location>> {
        sqlx::query_as::<_, Location>("SELECT * FROM locations WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn get_location_by_write_token(&self, token: &str) -> Result<Option<Location>> {
        sqlx::query_as::<_, Location>(
            "SELECT * FROM locations WHERE write_token = ? AND status != 'active'",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }

    #[allow(dead_code)]
    pub async fn mark_write_token_used(&self, token: &str) -> Result<SqliteQueryResult> {
        sqlx::query("UPDATE locations SET write_token_used = 1 WHERE write_token = ?")
            .bind(token)
            .execute(&self.pool)
            .await
            .map_err(Into::into)
    }

    /// List all locations regardless of status - useful for admin functionality
    pub async fn list_locations(&self) -> Result<Vec<Location>> {
        sqlx::query_as::<_, Location>("SELECT * FROM locations ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn list_active_locations(&self) -> Result<Vec<Location>> {
        sqlx::query_as::<_, Location>(
            "SELECT * FROM locations WHERE status = 'active' ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_locations_by_user(&self, user_id: &str) -> Result<Vec<Location>> {
        sqlx::query_as::<_, Location>(
            "SELECT * FROM locations WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update_location_msats(&self, id: &str, msats: i64) -> Result<SqliteQueryResult> {
        sqlx::query("UPDATE locations SET current_msats = ? WHERE id = ?")
            .bind(msats)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn update_last_refill(&self, id: &str) -> Result<SqliteQueryResult> {
        sqlx::query("UPDATE locations SET last_refill_at = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn update_location_status(
        &self,
        id: &str,
        status: &str,
    ) -> Result<SqliteQueryResult> {
        sqlx::query("UPDATE locations SET status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_location(&self, id: &str, user_id: &str) -> Result<SqliteQueryResult> {
        sqlx::query("DELETE FROM locations WHERE id = ? AND user_id = ? AND status != 'active'")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(Into::into)
    }

    // Photo operations
    pub async fn add_photo(&self, location_id: &str, file_path: String) -> Result<Photo> {
        let id = Uuid::new_v4().to_string();

        sqlx::query_as::<_, Photo>(
            "INSERT INTO photos (id, location_id, file_path, uploaded_at) VALUES (?, ?, ?, ?) RETURNING *"
        )
        .bind(&id)
        .bind(location_id)
        .bind(&file_path)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_photos_for_location(&self, location_id: &str) -> Result<Vec<Photo>> {
        sqlx::query_as::<_, Photo>(
            "SELECT * FROM photos WHERE location_id = ? ORDER BY uploaded_at ASC",
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_photo(&self, photo_id: &str) -> Result<Option<Photo>> {
        sqlx::query_as::<_, Photo>("SELECT * FROM photos WHERE id = ?")
            .bind(photo_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_photo(&self, photo_id: &str) -> Result<SqliteQueryResult> {
        sqlx::query("DELETE FROM photos WHERE id = ?")
            .bind(photo_id)
            .execute(&self.pool)
            .await
            .map_err(Into::into)
    }

    // =========================================================================
    // Donation operations (unified donations table)
    // =========================================================================

    /// Create a new donation when an invoice is generated
    pub async fn create_donation(
        &self,
        invoice: String,
        amount_msats: i64,
        location_id: Option<&str>,
    ) -> Result<Donation> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query_as::<_, Donation>(
            r#"
            INSERT INTO donations (id, location_id, invoice, amount_msats, status, created_at)
            VALUES (?, ?, ?, ?, 'created', ?)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(location_id)
        .bind(&invoice)
        .bind(amount_msats)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    /// Get a donation by invoice
    pub async fn get_donation_by_invoice(&self, invoice: &str) -> Result<Option<Donation>> {
        sqlx::query_as::<_, Donation>("SELECT * FROM donations WHERE invoice = ?")
            .bind(invoice)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    /// Mark a donation as received.
    /// For global donations (location_id = NULL), splits the amount equally among all active locations.
    pub async fn mark_donation_received(&self, invoice: &str) -> Result<Donation> {
        let now = Utc::now();

        // First, get the donation to check if it's global
        let donation: Donation = sqlx::query_as(
            "UPDATE donations SET status = 'received', received_at = ? WHERE invoice = ? RETURNING *",
        )
        .bind(now)
        .bind(invoice)
        .fetch_one(&self.pool)
        .await?;

        // If it's a global donation, split it among all active locations
        if donation.location_id.is_none() {
            let locations = self.list_active_locations().await?;
            if !locations.is_empty() {
                let amount_per_location = donation.amount_msats / locations.len() as i64;
                if amount_per_location > 0 {
                    for location in &locations {
                        let split_id = Uuid::new_v4().to_string();
                        sqlx::query(
                            "INSERT INTO donations (id, location_id, invoice, amount_msats, status, created_at, received_at) \
                             VALUES (?, ?, ?, ?, 'received', ?, ?)",
                        )
                        .bind(&split_id)
                        .bind(&location.id)
                        .bind(format!("{}-split-{}", invoice, location.id))
                        .bind(amount_per_location)
                        .bind(now)
                        .bind(now)
                        .execute(&self.pool)
                        .await?;
                    }
                }
            }
        }

        Ok(donation)
    }

    /// Mark a donation as timed out
    #[allow(dead_code)]
    pub async fn mark_donation_timed_out(&self, invoice: &str) -> Result<Donation> {
        sqlx::query_as::<_, Donation>(
            "UPDATE donations SET status = 'timed_out' WHERE invoice = ? RETURNING *",
        )
        .bind(invoice)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    /// List all pending (created) donations
    pub async fn list_pending_donations(&self) -> Result<Vec<Donation>> {
        sqlx::query_as::<_, Donation>(
            "SELECT * FROM donations WHERE status = 'created' ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    /// Get the balance of a location's donation pool
    /// (sum of received location donations minus debits)
    pub async fn get_location_donation_pool_balance(&self, location_id: &str) -> Result<i64> {
        // Sum of received donations for this location
        let donations: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(amount_msats), 0) FROM donations WHERE location_id = ? AND status = 'received'",
        )
        .bind(location_id)
        .fetch_one(&self.pool)
        .await?;

        // Sum of debits from this location's pool
        let debits: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(amount_msats), 0) FROM location_pool_debits WHERE location_id = ?",
        )
        .bind(location_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(donations.0 - debits.0)
    }

    /// Record a debit from a location's donation pool (when refills use the pool)
    pub async fn record_location_pool_debit(
        &self,
        location_id: &str,
        amount_msats: i64,
    ) -> Result<LocationPoolDebit> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query_as::<_, LocationPoolDebit>(
            r#"
            INSERT INTO location_pool_debits (id, location_id, amount_msats, created_at)
            VALUES (?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(location_id)
        .bind(amount_msats)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    /// List all received donations for a location (for display on location page)
    pub async fn list_location_donations(&self, location_id: &str) -> Result<Vec<Donation>> {
        sqlx::query_as::<_, Donation>(
            r#"
            SELECT * FROM donations
            WHERE location_id = ? AND status = 'received'
            ORDER BY received_at DESC
            "#,
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    /// List all received donations (global and location-specific), excluding split entries.
    ///
    /// Global donations create split entries with invoice format "{original}-split-{location_id}".
    /// We filter these out to show one entry per actual donation:
    /// - Global donations show with their full amount (location_id IS NULL)
    /// - Per-location donations show as-is (no splits created for them)
    pub async fn list_all_received_donations(&self, limit: i64) -> Result<Vec<Donation>> {
        sqlx::query_as::<_, Donation>(
            r#"
            SELECT * FROM donations
            WHERE status = 'received'
              AND invoice NOT LIKE '%-split-%'
            ORDER BY received_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    // Scan operations
    pub async fn record_scan(
        &self,
        location_id: &str,
        msats_withdrawn: i64,
        user_id: Option<&str>,
    ) -> Result<Scan> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Update last_withdraw_at on the location
        sqlx::query("UPDATE locations SET last_withdraw_at = ? WHERE id = ?")
            .bind(now)
            .bind(location_id)
            .execute(&self.pool)
            .await?;

        sqlx::query_as::<_, Scan>(
            "INSERT INTO scans (id, location_id, msats_withdrawn, scanned_at, user_id) VALUES (?, ?, ?, ?, ?) RETURNING *"
        )
        .bind(&id)
        .bind(location_id)
        .bind(msats_withdrawn)
        .bind(now)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_scans_for_location(&self, location_id: &str) -> Result<Vec<Scan>> {
        sqlx::query_as::<_, Scan>(
            "SELECT * FROM scans WHERE location_id = ? ORDER BY scanned_at DESC",
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    // Stats operations
    pub async fn get_stats(&self) -> Result<Stats> {
        let total_locations: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM locations WHERE status = 'active'")
                .fetch_one(&self.pool)
                .await?;

        let total_msats_available: Option<i64> =
            sqlx::query_scalar("SELECT SUM(current_msats) FROM locations WHERE status = 'active'")
                .fetch_one(&self.pool)
                .await?;

        let total_scans: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scans")
            .fetch_one(&self.pool)
            .await?;

        // Total donation pool = sum of all location-specific donations minus debits
        let total_donations: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(amount_msats), 0) FROM donations WHERE location_id IS NOT NULL AND status = 'received'",
        )
        .fetch_one(&self.pool)
        .await?;

        let total_debits: (i64,) =
            sqlx::query_as("SELECT COALESCE(SUM(amount_msats), 0) FROM location_pool_debits")
                .fetch_one(&self.pool)
                .await?;

        let total_pool_msats = total_donations.0 - total_debits.0;

        Ok(Stats {
            total_locations,
            total_sats_available: total_msats_available.unwrap_or(0) / 1000, // Convert to sats for display
            total_scans,
            donation_pool_sats: total_pool_msats / 1000,
        })
    }

    // NFC card operations
    pub async fn create_nfc_card(
        &self,
        location_id: String,
        k0_auth_key: String,
        k1_decrypt_key: String,
        k2_cmac_key: String,
        k3: String,
        k4: String,
    ) -> Result<NfcCard> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query_as::<_, NfcCard>(
            r#"
            INSERT INTO nfc_cards (
                id, location_id, k0_auth_key, k1_decrypt_key, k2_cmac_key, k3, k4,
                counter, version, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, 0, 0, ?)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&location_id)
        .bind(&k0_auth_key)
        .bind(&k1_decrypt_key)
        .bind(&k2_cmac_key)
        .bind(&k3)
        .bind(&k4)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_nfc_card_by_location(&self, location_id: &str) -> Result<Option<NfcCard>> {
        sqlx::query_as::<_, NfcCard>("SELECT * FROM nfc_cards WHERE location_id = ?")
            .bind(location_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    /// Get NFC card by UID - will be used for payment verification with NFC taps
    #[allow(dead_code)]
    pub async fn get_nfc_card_by_uid(&self, uid: &str) -> Result<Option<NfcCard>> {
        sqlx::query_as::<_, NfcCard>("SELECT * FROM nfc_cards WHERE uid = ?")
            .bind(uid)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn update_nfc_card_uid_and_mark_programmed(
        &self,
        location_id: &str,
        uid: &str,
    ) -> Result<SqliteQueryResult> {
        sqlx::query("UPDATE nfc_cards SET uid = ?, programmed_at = ? WHERE location_id = ?")
            .bind(uid)
            .bind(Utc::now())
            .bind(location_id)
            .execute(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn increment_nfc_card_version(&self, location_id: &str) -> Result<SqliteQueryResult> {
        sqlx::query("UPDATE nfc_cards SET version = version + 1 WHERE location_id = ?")
            .bind(location_id)
            .execute(&self.pool)
            .await
            .map_err(Into::into)
    }

    /// Atomically claim a withdrawal by updating the counter and zeroing the balance.
    ///
    /// This prevents double-spending by checking the counter hasn't been used yet
    /// and updating it in a single transaction. Returns the claimed amount in msats
    /// if successful, or None if the counter was already claimed.
    pub async fn claim_withdrawal(
        &self,
        location_id: &str,
        new_counter: i64,
    ) -> Result<Option<i64>> {
        let mut tx = self.pool.begin().await?;

        // Check if counter is still valid (not already claimed)
        let card: Option<NfcCard> = sqlx::query_as("SELECT * FROM nfc_cards WHERE location_id = ?")
            .bind(location_id)
            .fetch_optional(&mut *tx)
            .await?;

        let card = match card {
            Some(c) => c,
            None => return Ok(None),
        };

        // If counter has already been claimed, reject
        if new_counter <= card.counter {
            return Ok(None);
        }

        // Get the current balance
        let location: Option<Location> = sqlx::query_as("SELECT * FROM locations WHERE id = ?")
            .bind(location_id)
            .fetch_optional(&mut *tx)
            .await?;

        let location = match location {
            Some(l) => l,
            None => return Ok(None),
        };

        let withdrawable_msats = location.withdrawable_msats();
        if withdrawable_msats <= 0 {
            return Ok(None);
        }

        // Update the counter
        sqlx::query("UPDATE nfc_cards SET counter = ?, last_used_at = ? WHERE location_id = ?")
            .bind(new_counter)
            .bind(Utc::now())
            .bind(location_id)
            .execute(&mut *tx)
            .await?;

        // Zero the balance
        sqlx::query("UPDATE locations SET current_msats = 0 WHERE id = ?")
            .bind(location_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(Some(withdrawable_msats))
    }

    /// Update NFC card counter (for non-withdrawal scans like activation)
    pub async fn update_nfc_card_counter(
        &self,
        location_id: &str,
        counter: i64,
    ) -> Result<SqliteQueryResult> {
        sqlx::query("UPDATE nfc_cards SET counter = ?, last_used_at = ? WHERE location_id = ?")
            .bind(counter)
            .bind(Utc::now())
            .bind(location_id)
            .execute(&self.pool)
            .await
            .map_err(Into::into)
    }

    // Refill operations
    pub async fn record_refill(
        &self,
        location_id: &str,
        msats_added: i64,
        balance_before_msats: i64,
        balance_after_msats: i64,
        base_rate_msats_per_min: i64,
        slowdown_factor: f64,
    ) -> Result<Refill> {
        let id = Uuid::new_v4().to_string();

        sqlx::query_as::<_, Refill>(
            r#"
            INSERT INTO refills (
                id, location_id, msats_added, balance_before_msats, balance_after_msats,
                base_rate_msats_per_min, slowdown_factor, refilled_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(location_id)
        .bind(msats_added)
        .bind(balance_before_msats)
        .bind(balance_after_msats)
        .bind(base_rate_msats_per_min)
        .bind(slowdown_factor)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_refills_for_location(&self, location_id: &str) -> Result<Vec<Refill>> {
        sqlx::query_as::<_, Refill>(
            "SELECT * FROM refills WHERE location_id = ? ORDER BY refilled_at DESC LIMIT 100",
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    // ========================================================================
    // Anonymous User and Custodial Wallet Operations
    // ========================================================================

    /// Create an anonymous user (lazy creation on first collection)
    pub async fn create_anonymous_user(&self, id: &str) -> Result<User> {
        let now = Utc::now();
        let auth_method = AuthMethod::Anonymous {};
        let method_type = auth_method.to_type_string();
        let method_data = auth_method.to_json()?;

        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, email, auth_method, auth_data, created_at, role)
            VALUES (?, NULL, NULL, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(method_type)
        .bind(&method_data)
        .bind(now)
        .bind(UserRole::User.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    /// Get or create an anonymous user - used during collection
    pub async fn get_or_create_anonymous_user(&self, id: &str) -> Result<User> {
        // Try to get existing user first
        if let Some(user) = self.get_user_by_id(id).await? {
            return Ok(user);
        }

        // Create new anonymous user
        self.create_anonymous_user(id).await
    }

    /// Get user's balance (sum of collections - sum of withdrawals)
    pub async fn get_user_balance(&self, user_id: &str) -> Result<i64> {
        // Get balance from transactions
        let tx_balance: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(
                SUM(CASE WHEN transaction_type = 'collect' THEN msats ELSE -msats END),
                0
            ) FROM user_transactions WHERE user_id = ?
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Get pending withdrawals (reserved but not yet completed)
        let pending: Option<i64> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(msats), 0) FROM pending_withdrawals WHERE user_id = ? AND status = ?",
        )
        .bind(user_id)
        .bind(WithdrawalStatus::Pending.as_str())
        .fetch_one(&self.pool)
        .await?;

        // Available balance = transactions balance - pending withdrawals
        Ok(tx_balance.unwrap_or(0) - pending.unwrap_or(0))
    }

    /// Get user's transaction history
    pub async fn get_user_transactions(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<UserTransaction>> {
        sqlx::query_as::<_, UserTransaction>(
            "SELECT * FROM user_transactions WHERE user_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    /// Atomically claim a collection - takes sats from location and credits to user.
    ///
    /// This is the core operation for the custodial wallet system. It:
    /// 1. Verifies the NFC counter hasn't been used (replay protection)
    /// 2. Creates the anonymous user if they don't exist (lazy creation)
    /// 3. Zeros the location balance
    /// 4. Records a collection transaction for the user
    ///
    /// Returns the collected amount in msats, or None if the counter was already used.
    pub async fn claim_collection(
        &self,
        location_id: &str,
        user_id: &str,
        new_counter: i64,
    ) -> Result<Option<i64>> {
        let mut tx = self.pool.begin().await?;

        // Check if counter is still valid (not already claimed)
        let card: Option<NfcCard> = sqlx::query_as("SELECT * FROM nfc_cards WHERE location_id = ?")
            .bind(location_id)
            .fetch_optional(&mut *tx)
            .await?;

        let card = match card {
            Some(c) => c,
            None => return Ok(None),
        };

        // If counter has already been claimed, reject
        if new_counter <= card.counter {
            return Ok(None);
        }

        // Get the current balance (no fee deduction for custodial collection)
        let location: Option<Location> = sqlx::query_as("SELECT * FROM locations WHERE id = ?")
            .bind(location_id)
            .fetch_optional(&mut *tx)
            .await?;

        let location = match location {
            Some(l) => l,
            None => return Ok(None),
        };

        let collected_msats = location.current_msats;
        if collected_msats <= 0 {
            return Ok(None);
        }

        let now = Utc::now();

        // Update the counter
        sqlx::query("UPDATE nfc_cards SET counter = ?, last_used_at = ? WHERE location_id = ?")
            .bind(new_counter)
            .bind(now)
            .bind(location_id)
            .execute(&mut *tx)
            .await?;

        // Zero the location balance
        sqlx::query("UPDATE locations SET current_msats = 0, last_withdraw_at = ? WHERE id = ?")
            .bind(now)
            .bind(location_id)
            .execute(&mut *tx)
            .await?;

        // Ensure user exists (lazy creation)
        let user_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = ?)")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?;

        if !user_exists {
            sqlx::query(
                "INSERT INTO users (id, username, email, auth_method, auth_data, created_at, role) VALUES (?, NULL, NULL, 'anonymous', '{}', ?, 'user')"
            )
            .bind(user_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }

        // Record the collection transaction
        let tx_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO user_transactions (id, user_id, location_id, msats, transaction_type, created_at) VALUES (?, ?, ?, ?, 'collect', ?)"
        )
        .bind(&tx_id)
        .bind(user_id)
        .bind(location_id)
        .bind(collected_msats)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        // Record the scan with user_id
        let scan_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO scans (id, location_id, msats_withdrawn, scanned_at, user_id) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&scan_id)
        .bind(location_id)
        .bind(collected_msats)
        .bind(now)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Some(collected_msats))
    }

    // Settings operations

    /// Get a setting value by key
    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let result: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(result.map(|(v,)| v))
    }

    /// Set a setting value, inserting or updating as needed
    pub async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO settings (key, value, created_at, updated_at) VALUES (?, ?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(value)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Create a pending withdrawal, reserving the balance.
    ///
    /// This creates a pending withdrawal record that reduces the user's available balance.
    /// The `fee_msats` parameter specifies the fees to charge on top of the invoice amount.
    /// Returns the pending withdrawal ID if successful, or None if insufficient balance.
    pub async fn create_pending_withdrawal(
        &self,
        user_id: &str,
        amount_msats: i64,
        fee_msats: i64,
        invoice: &str,
    ) -> Result<Option<String>> {
        let total_msats = amount_msats + fee_msats;
        let mut tx = self.pool.begin().await?;

        // Get current balance from transactions
        let tx_balance: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(
                SUM(CASE WHEN transaction_type = 'collect' THEN msats ELSE -msats END),
                0
            ) FROM user_transactions WHERE user_id = ?
            "#,
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        // Get existing pending withdrawals
        let pending: Option<i64> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(msats), 0) FROM pending_withdrawals WHERE user_id = ? AND status = ?",
        )
        .bind(user_id)
        .bind(WithdrawalStatus::Pending.as_str())
        .fetch_one(&mut *tx)
        .await?;

        let available_balance = tx_balance.unwrap_or(0) - pending.unwrap_or(0);

        // Check sufficient balance for amount + fees
        if available_balance < total_msats {
            return Ok(None);
        }

        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        // Create pending withdrawal (reserves amount + fees)
        sqlx::query(
            "INSERT INTO pending_withdrawals (id, user_id, msats, invoice, status, created_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(user_id)
        .bind(total_msats)
        .bind(invoice)
        .bind(WithdrawalStatus::Pending.as_str())
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Some(id))
    }

    /// Complete a pending withdrawal, recording the actual transaction.
    ///
    /// This marks the pending withdrawal as completed and records the withdrawal transaction.
    pub async fn complete_pending_withdrawal(&self, withdrawal_id: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();

        // Get the pending withdrawal
        let withdrawal: Option<(String, i64)> = sqlx::query_as(
            "SELECT user_id, msats FROM pending_withdrawals WHERE id = ? AND status = ?",
        )
        .bind(withdrawal_id)
        .bind(WithdrawalStatus::Pending.as_str())
        .fetch_optional(&mut *tx)
        .await?;

        let (user_id, msats) = withdrawal
            .ok_or_else(|| anyhow::anyhow!("Pending withdrawal not found or already processed"))?;

        // Mark as completed
        sqlx::query("UPDATE pending_withdrawals SET status = ?, completed_at = ? WHERE id = ?")
            .bind(WithdrawalStatus::Completed.as_str())
            .bind(now)
            .bind(withdrawal_id)
            .execute(&mut *tx)
            .await?;

        // Record the withdrawal transaction
        let tx_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO user_transactions (id, user_id, location_id, msats, transaction_type, created_at) VALUES (?, ?, NULL, ?, 'withdraw', ?)"
        )
        .bind(&tx_id)
        .bind(&user_id)
        .bind(msats)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    /// Fail a pending withdrawal, releasing the reserved balance.
    ///
    /// This marks the pending withdrawal as failed, making the balance available again.
    pub async fn fail_pending_withdrawal(&self, withdrawal_id: &str) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            "UPDATE pending_withdrawals SET status = ?, completed_at = ? WHERE id = ? AND status = ?",
        )
        .bind(WithdrawalStatus::Failed.as_str())
        .bind(now)
        .bind(withdrawal_id)
        .bind(WithdrawalStatus::Pending.as_str())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get or create the cookie secret for private cookie jar.
    /// Generates a random 64-byte secret on first use (required by axum-extra's Key).
    pub async fn get_or_create_cookie_secret(&self) -> Result<Vec<u8>> {
        const KEY: &str = "cookie_secret";

        if let Some(hex_secret) = self.get_setting(KEY).await? {
            // Decode existing secret from hex
            let secret = hex::decode(&hex_secret)?;
            // If we have an old 32-byte secret, regenerate with 64 bytes
            if secret.len() >= 64 {
                return Ok(secret);
            }
            tracing::warn!(
                "Cookie secret too short ({}), regenerating with 64 bytes",
                secret.len()
            );
        }

        // Generate new random secret (64 bytes required by axum-extra Key)
        use rand::{thread_rng, RngCore};
        let mut secret = vec![0u8; 64];
        thread_rng().fill_bytes(&mut secret);

        // Store as hex
        let hex_secret = hex::encode(&secret);
        self.set_setting(KEY, &hex_secret).await?;

        tracing::info!("Generated new cookie secret (64 bytes)");
        Ok(secret)
    }
}
