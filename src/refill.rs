use crate::db::Database;
use anyhow::Result;
use chrono::{Duration, Utc};
use std::sync::Arc;
use tokio::time;

/// Configuration for the refill service
pub struct RefillConfig {
    /// Sats per hour to add to each location
    pub sats_per_hour: i64,
    /// How often to run the refill check (in seconds)
    pub check_interval_secs: u64,
}

impl Default for RefillConfig {
    fn default() -> Self {
        Self {
            sats_per_hour: 100,
            check_interval_secs: 300, // 5 minutes
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

    /// Refill all locations that are due for a refill
    async fn refill_locations(&self) -> Result<()> {
        let locations = self.db.list_locations().await?;
        let donation_pool = self.db.get_donation_pool().await?;

        let now = Utc::now();
        let mut total_refilled = 0i64;

        for location in locations {
            // Calculate how much time has passed since last refill
            let hours_since_refill = (now - location.last_refill_at).num_hours();

            if hours_since_refill < 1 {
                continue; // Not time to refill yet
            }

            // Calculate refill amount
            let refill_amount = hours_since_refill * self.config.sats_per_hour;
            let new_balance = (location.current_sats + refill_amount).min(location.max_sats);
            let actual_refill = new_balance - location.current_sats;

            if actual_refill <= 0 {
                continue; // Already at max
            }

            // Check if donation pool has enough
            if donation_pool.total_sats < actual_refill {
                tracing::warn!(
                    "Donation pool too low to refill location {}: need {}, have {}",
                    location.name,
                    actual_refill,
                    donation_pool.total_sats
                );
                continue;
            }

            // Update location balance
            self.db.update_location_sats(&location.id, new_balance).await?;
            self.db.update_last_refill(&location.id).await?;

            total_refilled += actual_refill;

            tracing::info!(
                "Refilled location {} with {} sats (now at {}/{})",
                location.name,
                actual_refill,
                new_balance,
                location.max_sats
            );
        }

        // Subtract from donation pool
        if total_refilled > 0 {
            self.db.subtract_from_donation_pool(total_refilled).await?;
            tracing::info!(
                "Total refilled: {} sats, remaining pool: {} sats",
                total_refilled,
                donation_pool.total_sats - total_refilled
            );
        }

        Ok(())
    }
}
