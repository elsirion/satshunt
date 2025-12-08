use crate::db::Database;
use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use tokio::time;

/// Configuration for the refill service
pub struct RefillConfig {
    /// Sats per hour to add to each location
    pub sats_per_hour: i64,
    /// How often to run the refill check (in seconds)
    pub check_interval_secs: u64,
    /// Maximum sats per location (global cap)
    pub max_sats_per_location: i64,
}

impl Default for RefillConfig {
    fn default() -> Self {
        Self {
            sats_per_hour: 100,
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

    /// Refill all active locations that are due for a refill
    async fn refill_locations(&self) -> Result<()> {
        let locations = self.db.list_active_locations().await?;
        let donation_pool = self.db.get_donation_pool().await?;

        let now = Utc::now();
        let mut total_refilled_msats = 0i64;
        let mut remaining_pool_msats = donation_pool.total_msats;

        for location in locations {
            // Calculate how much time has passed since last refill in minutes
            let minutes_since_refill = (now - location.last_refill_at).num_minutes();

            if minutes_since_refill < 1 {
                continue; // Not time to refill yet
            }

            // Calculate refill amount (1 sat per minute = 1000 msats per minute)
            let msats_per_minute = (self.config.sats_per_hour as f64 / 60.0 * 1000.0).round() as i64;
            let refill_amount_msats = minutes_since_refill * msats_per_minute;
            let max_msats = self.config.max_sats_per_location * 1000;
            let new_balance_msats = (location.current_msats + refill_amount_msats).min(max_msats);
            let actual_refill_msats = new_balance_msats - location.current_msats;

            if actual_refill_msats <= 0 {
                continue; // Already at max
            }

            // Check if remaining pool has enough
            if remaining_pool_msats < actual_refill_msats {
                tracing::warn!(
                    "Donation pool too low to refill location {}: need {} msats, have {} msats",
                    location.name,
                    actual_refill_msats,
                    remaining_pool_msats
                );
                continue;
            }

            // Update location balance
            self.db.update_location_msats(&location.id, new_balance_msats).await?;
            self.db.update_last_refill(&location.id).await?;

            total_refilled_msats += actual_refill_msats;
            remaining_pool_msats -= actual_refill_msats;

            tracing::info!(
                "Refilled location {} with {} sats (now at {}/{})",
                location.name,
                actual_refill_msats / 1000,
                new_balance_msats / 1000,
                self.config.max_sats_per_location
            );
        }

        // Subtract from donation pool
        if total_refilled_msats > 0 {
            self.db.subtract_from_donation_pool(total_refilled_msats).await?;
            tracing::info!(
                "Total refilled: {} sats, remaining pool: {} sats",
                total_refilled_msats / 1000,
                remaining_pool_msats / 1000
            );
        }

        Ok(())
    }
}
