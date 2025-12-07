use crate::models::*;
use anyhow::Result;
use chrono::Utc;
use sqlx::{SqlitePool, sqlite::{SqliteConnectOptions, SqliteQueryResult}};
use uuid::Uuid;
use std::str::FromStr;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        // Configure SQLite to create the database file if it doesn't exist
        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true);

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
            INSERT INTO users (id, username, email, auth_method, auth_data, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&username)
        .bind(&email)
        .bind(method_type)
        .bind(&method_data)
        .bind(now)
        .fetch_one(&self.pool)
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
                current_sats, lnurlw_secret,
                created_at, last_refill_at, write_token, write_token_created_at, user_id
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&name)
        .bind(latitude)
        .bind(longitude)
        .bind(&description)
        .bind(0) // current_sats starts at 0
        .bind(&lnurlw_secret)
        .bind(now)
        .bind(now)
        .bind(&write_token)
        .bind(now)
        .bind(&user_id)
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
            "SELECT * FROM locations WHERE write_token = ? AND write_token_used = 0"
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

    pub async fn list_locations(&self) -> Result<Vec<Location>> {
        sqlx::query_as::<_, Location>("SELECT * FROM locations ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn get_locations_by_user(&self, user_id: &str) -> Result<Vec<Location>> {
        sqlx::query_as::<_, Location>(
            "SELECT * FROM locations WHERE user_id = ? ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update_location_sats(&self, id: &str, sats: i64) -> Result<SqliteQueryResult> {
        sqlx::query("UPDATE locations SET current_sats = ? WHERE id = ?")
            .bind(sats)
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
            "SELECT * FROM photos WHERE location_id = ? ORDER BY uploaded_at ASC"
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    // Donation pool operations
    pub async fn get_donation_pool(&self) -> Result<DonationPool> {
        sqlx::query_as::<_, DonationPool>("SELECT * FROM donation_pool WHERE id = 1")
            .fetch_one(&self.pool)
            .await
            .map_err(Into::into)
    }

    #[allow(dead_code)]
    pub async fn update_donation_pool(&self, sats: i64) -> Result<SqliteQueryResult> {
        sqlx::query("UPDATE donation_pool SET total_sats = ?, updated_at = ? WHERE id = 1")
            .bind(sats)
            .bind(Utc::now())
            .execute(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn add_to_donation_pool(&self, sats: i64) -> Result<DonationPool> {
        sqlx::query_as::<_, DonationPool>(
            "UPDATE donation_pool SET total_sats = total_sats + ?, updated_at = ? WHERE id = 1 RETURNING *"
        )
        .bind(sats)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn subtract_from_donation_pool(&self, sats: i64) -> Result<DonationPool> {
        sqlx::query_as::<_, DonationPool>(
            "UPDATE donation_pool SET total_sats = total_sats - ?, updated_at = ? WHERE id = 1 RETURNING *"
        )
        .bind(sats)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    // Scan operations
    pub async fn record_scan(&self, location_id: &str, sats_withdrawn: i64) -> Result<Scan> {
        let id = Uuid::new_v4().to_string();

        sqlx::query_as::<_, Scan>(
            "INSERT INTO scans (id, location_id, sats_withdrawn, scanned_at) VALUES (?, ?, ?, ?) RETURNING *"
        )
        .bind(&id)
        .bind(location_id)
        .bind(sats_withdrawn)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    // Stats operations
    pub async fn get_stats(&self) -> Result<Stats> {
        let total_locations: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM locations")
            .fetch_one(&self.pool)
            .await?;

        let total_sats_available: Option<i64> =
            sqlx::query_scalar("SELECT SUM(current_sats) FROM locations")
                .fetch_one(&self.pool)
                .await?;

        let total_scans: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scans")
            .fetch_one(&self.pool)
            .await?;

        let donation_pool = self.get_donation_pool().await?;

        Ok(Stats {
            total_locations,
            total_sats_available: total_sats_available.unwrap_or(0),
            total_scans,
            donation_pool_sats: donation_pool.total_sats,
        })
    }
}
