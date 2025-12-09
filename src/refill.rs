use crate::db::Database;
use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use tokio::time;

/// Configuration for the refill service
pub struct RefillConfig {
    /// Percentage of donation pool to distribute per minute (default: 0.016%)
    pub pool_percentage_per_minute: f64,
    /// How often to run the refill check (in seconds)
    pub check_interval_secs: u64,
    /// Maximum sats per location (global cap)
    pub max_sats_per_location: i64,
}

impl Default for RefillConfig {
    fn default() -> Self {
        Self {
            pool_percentage_per_minute: 0.00016, // 0.016% of pool per minute
            check_interval_secs: 300, // 5 minutes
            max_sats_per_location: 1000,
        }
    }
}

/// Background service that refills locations from the donation pool
pub struct RefillService {
    db: Arc<Database>,
    config: RefillConfig,
}

impl RefillService {
    pub fn new(db: Arc<Database>, config: RefillConfig) -> Self {
        Self { db, config }
    }

    /// Get the maximum sats per location from config
    #[allow(dead_code)]
    pub fn max_sats_per_location(&self) -> i64 {
        self.config.max_sats_per_location
    }

    /// Start the refill service
    pub async fn start(self: Arc<Self>) {
        let mut interval = time::interval(time::Duration::from_secs(self.config.check_interval_secs));

        loop {
            interval.tick().await;

            if let Err(e) = self.refill_locations().await {
                tracing::error!("Error during refill: {}", e);
            }
        }
    }

    /// Calculate slowdown factor based on how full the location is
    /// Formula: slowdown = 1 / (1 + exp(k * (fill_ratio - 0.8)))
    /// As location fills up past 80%, refill rate slows down
    fn calculate_slowdown_factor(current_msats: i64, max_msats: i64) -> f64 {
        const K: f64 = 0.1; // steepness parameter
        const THRESHOLD: f64 = 0.8; // start slowing down at 80% full

        let fill_ratio = current_msats as f64 / max_msats as f64;
        let exponent = K * (fill_ratio - THRESHOLD);
        1.0 / (1.0 + exponent.exp())
    }

    /// Refill all active locations that are due for a refill
    /// Uses formula: refill_per_location = (pool * 0.00016) / num_locations per minute
    /// With slowdown as location fills up
    async fn refill_locations(&self) -> Result<()> {
        let locations = self.db.list_active_locations().await?;
        let num_locations = locations.len();

        if num_locations == 0 {
            return Ok(()); // No locations to refill
        }

        let donation_pool = self.db.get_donation_pool().await?;
        let now = Utc::now();
        let mut total_refilled_msats = 0i64;

        // Calculate base refill rate per location per minute based on pool size
        // Formula: (pool * percentage) / num_locations
        let base_msats_per_location_per_minute =
            ((donation_pool.total_msats as f64 * self.config.pool_percentage_per_minute) / num_locations as f64).round() as i64;

        tracing::debug!(
            "Base refill rate: {} msats per location per minute (pool: {} msats, locations: {})",
            base_msats_per_location_per_minute,
            donation_pool.total_msats,
            num_locations
        );

        for location in locations {
            // Calculate how much time has passed since last refill in minutes
            let minutes_since_refill = (now - location.last_refill_at).num_minutes();

            if minutes_since_refill < 1 {
                continue; // Not time to refill yet
            }

            let max_msats = self.config.max_sats_per_location * 1000;

            // Apply slowdown factor based on how full the location is
            let slowdown_factor = Self::calculate_slowdown_factor(location.current_msats, max_msats);
            let adjusted_rate_msats = (base_msats_per_location_per_minute as f64 * slowdown_factor).round() as i64;

            // Calculate refill amount based on minutes elapsed and adjusted rate
            let refill_amount_msats = minutes_since_refill * adjusted_rate_msats;
            let new_balance_msats = (location.current_msats + refill_amount_msats).min(max_msats);
            let actual_refill_msats = new_balance_msats - location.current_msats;

            if actual_refill_msats <= 0 {
                continue; // Already at max
            }

            let balance_before = location.current_msats;

            // Update location balance
            self.db.update_location_msats(&location.id, new_balance_msats).await?;
            self.db.update_last_refill(&location.id).await?;

            // Record the refill in the log
            self.db.record_refill(
                &location.id,
                actual_refill_msats,
                balance_before,
                new_balance_msats,
                base_msats_per_location_per_minute,
                slowdown_factor,
            ).await?;

            total_refilled_msats += actual_refill_msats;

            tracing::info!(
                "Refilled location {} with {} sats (now at {}/{}, rate: {} sats/min, slowdown: {:.2}x)",
                location.name,
                actual_refill_msats / 1000,
                new_balance_msats / 1000,
                self.config.max_sats_per_location,
                adjusted_rate_msats / 1000,
                slowdown_factor
            );
        }

        // Subtract from donation pool
        if total_refilled_msats > 0 {
            let new_pool = self.db.subtract_from_donation_pool(total_refilled_msats).await?;
            tracing::info!(
                "Total refilled: {} sats across {} locations, pool now: {} sats",
                total_refilled_msats / 1000,
                num_locations,
                new_pool.total_msats / 1000
            );
        }

        Ok(())
    }
}
