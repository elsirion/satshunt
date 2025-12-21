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
            check_interval_secs: 300,            // 5 minutes
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
        let mut interval =
            time::interval(time::Duration::from_secs(self.config.check_interval_secs));

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
    ///
    /// Returns a value between 0 and 1:
    /// - Near 1.0 when location is empty to ~80% full
    /// - Decreases towards 0 as location approaches 100% full
    pub fn calculate_slowdown_factor(current_msats: i64, max_msats: i64) -> f64 {
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
        let base_msats_per_location_per_minute = ((donation_pool.total_msats as f64
            * self.config.pool_percentage_per_minute)
            / num_locations as f64)
            .round() as i64;

        tracing::debug!(
            "Base refill rate: {} msats per location per minute (pool: {} msats, locations: {})",
            base_msats_per_location_per_minute,
            donation_pool.total_msats,
            num_locations
        );

        for location in locations {
            // Calculate how much time has passed since last activity (refill or withdraw)
            // We use the smaller delta (more recent activity) to avoid gaming
            let minutes_since_activity = (now - location.last_activity_at()).num_minutes();

            if minutes_since_activity < 1 {
                continue; // Not time to refill yet
            }

            let max_msats = self.config.max_sats_per_location * 1000;

            // Apply slowdown factor based on how full the location is
            let slowdown_factor =
                Self::calculate_slowdown_factor(location.current_msats, max_msats);
            let adjusted_rate_msats =
                (base_msats_per_location_per_minute as f64 * slowdown_factor).round() as i64;

            // Calculate refill amount based on minutes elapsed and adjusted rate
            let refill_amount_msats = minutes_since_activity * adjusted_rate_msats;
            let new_balance_msats = (location.current_msats + refill_amount_msats).min(max_msats);
            let actual_refill_msats = new_balance_msats - location.current_msats;

            if actual_refill_msats <= 0 {
                continue; // Already at max
            }

            let balance_before = location.current_msats;

            // Update location balance
            self.db
                .update_location_msats(&location.id, new_balance_msats)
                .await?;
            self.db.update_last_refill(&location.id).await?;

            // Record the refill in the log
            self.db
                .record_refill(
                    &location.id,
                    actual_refill_msats,
                    balance_before,
                    new_balance_msats,
                    base_msats_per_location_per_minute,
                    slowdown_factor,
                )
                .await?;

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
            let new_pool = self
                .db
                .subtract_from_donation_pool(total_refilled_msats)
                .await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slowdown_factor_empty() {
        // Empty location (0% full) should have factor close to 1.0
        let factor = RefillService::calculate_slowdown_factor(0, 1000000);
        // At 0% fill, exponent = 0.1 * (0 - 0.8) = -0.08
        // factor = 1 / (1 + e^-0.08) ≈ 0.52
        assert!(factor > 0.5);
        assert!(factor < 0.55);
    }

    #[test]
    fn test_slowdown_factor_half_full() {
        // 50% full should still have a decent factor
        let factor = RefillService::calculate_slowdown_factor(500000, 1000000);
        // At 50% fill, exponent = 0.1 * (0.5 - 0.8) = -0.03
        // factor = 1 / (1 + e^-0.03) ≈ 0.5075
        assert!(factor > 0.5);
        assert!(factor < 0.52);
    }

    #[test]
    fn test_slowdown_factor_at_threshold() {
        // Exactly at 80% threshold
        let factor = RefillService::calculate_slowdown_factor(800000, 1000000);
        // At 80% fill, exponent = 0.1 * (0.8 - 0.8) = 0
        // factor = 1 / (1 + e^0) = 1 / 2 = 0.5
        assert!((factor - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_slowdown_factor_ninety_percent() {
        // 90% full should start showing slowdown
        let factor = RefillService::calculate_slowdown_factor(900000, 1000000);
        // At 90% fill, exponent = 0.1 * (0.9 - 0.8) = 0.01
        // factor = 1 / (1 + e^0.01) ≈ 0.4975
        assert!(factor < 0.5);
        assert!(factor > 0.49);
    }

    #[test]
    fn test_slowdown_factor_full() {
        // 100% full
        let factor = RefillService::calculate_slowdown_factor(1000000, 1000000);
        // At 100% fill, exponent = 0.1 * (1.0 - 0.8) = 0.02
        // factor = 1 / (1 + e^0.02) ≈ 0.495
        assert!(factor < 0.5);
        assert!(factor > 0.48);
    }

    #[test]
    fn test_slowdown_factor_overfull() {
        // Over 100% (edge case, shouldn't happen but let's be safe)
        let factor = RefillService::calculate_slowdown_factor(1500000, 1000000);
        // At 150% fill, exponent = 0.1 * (1.5 - 0.8) = 0.07
        // factor = 1 / (1 + e^0.07) ≈ 0.4825
        assert!(factor < 0.49);
        assert!(factor > 0.47);
    }

    #[test]
    fn test_slowdown_factor_monotonic_decrease() {
        // Factor should decrease as fill ratio increases
        let f0 = RefillService::calculate_slowdown_factor(0, 1000000);
        let f50 = RefillService::calculate_slowdown_factor(500000, 1000000);
        let f80 = RefillService::calculate_slowdown_factor(800000, 1000000);
        let f100 = RefillService::calculate_slowdown_factor(1000000, 1000000);

        assert!(f0 > f50);
        assert!(f50 > f80);
        assert!(f80 > f100);
    }

    #[test]
    fn test_refill_config_default() {
        let config = RefillConfig::default();

        assert!((config.pool_percentage_per_minute - 0.00016).abs() < 0.00001);
        assert_eq!(config.check_interval_secs, 300);
        assert_eq!(config.max_sats_per_location, 1000);
    }
}
