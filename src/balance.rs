use chrono::{DateTime, Utc};

const SECS_PER_DAY: i64 = 24 * 60 * 60;

/// Configuration for balance calculation
#[derive(Debug, Clone)]
pub struct BalanceConfig {
    /// Time to fill from 0 to max_fill (in seconds)
    pub time_to_full_secs: i64,
    /// Maximum percentage of pool that can fill a location (e.g., 0.1 = 10%)
    pub max_fill_percentage: f64,
}

impl Default for BalanceConfig {
    fn default() -> Self {
        Self {
            time_to_full_secs: 21 * SECS_PER_DAY,
            max_fill_percentage: 0.1,
        }
    }
}

impl BalanceConfig {
    /// Apply optional per-location overrides on top of this default config.
    pub fn with_overrides(
        &self,
        time_to_full_secs: Option<i64>,
        max_fill_percentage: Option<f64>,
    ) -> Self {
        Self {
            time_to_full_secs: time_to_full_secs.unwrap_or(self.time_to_full_secs),
            max_fill_percentage: max_fill_percentage.unwrap_or(self.max_fill_percentage),
        }
    }
}

/// Calculate the computed balance for a location
///
/// Formula:
/// - max_fill = pool_balance * max_fill_percentage
/// - fill_ratio = min(time_since_withdraw / time_to_full, 1.0)
/// - computed_balance = max_fill * fill_ratio
///
/// Uses `created_at` when `last_withdraw_at` is None (location never withdrawn from).
pub fn compute_balance_msats(
    pool_balance_msats: i64,
    last_withdraw_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    config: &BalanceConfig,
) -> i64 {
    if pool_balance_msats <= 0 {
        return 0;
    }

    // Determine reference time (last withdraw or creation time)
    let reference_time = last_withdraw_at.unwrap_or(created_at);
    let now = Utc::now();

    // Calculate time elapsed since reference
    let elapsed = now.signed_duration_since(reference_time);
    let elapsed_secs = elapsed.num_seconds().max(0) as f64;

    // Time to full in seconds (guard against zero/negative to avoid division by zero)
    let time_to_full_secs = config.time_to_full_secs.max(1) as f64;

    // Fill ratio (0.0 to 1.0)
    let fill_ratio = (elapsed_secs / time_to_full_secs).min(1.0);

    // Max fill based on pool percentage
    let max_fill_msats = (pool_balance_msats as f64 * config.max_fill_percentage) as i64;

    // Computed balance
    (max_fill_msats as f64 * fill_ratio) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn test_config() -> BalanceConfig {
        BalanceConfig {
            time_to_full_secs: 21 * SECS_PER_DAY,
            max_fill_percentage: 0.1,
        }
    }

    #[test]
    fn test_empty_pool_returns_zero() {
        let config = test_config();
        let now = Utc::now();
        let result = compute_balance_msats(0, None, now, &config);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_negative_pool_returns_zero() {
        let config = test_config();
        let now = Utc::now();
        let result = compute_balance_msats(-1000, None, now, &config);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_new_location_starts_at_zero() {
        let config = test_config();
        let now = Utc::now();
        // Created just now, no withdrawals
        let result = compute_balance_msats(1_000_000_000, None, now, &config); // 1M sats pool
        assert_eq!(result, 0);
    }

    #[test]
    fn test_half_time_gives_half_fill() {
        let config = test_config();
        let now = Utc::now();
        let created_at = now - Duration::milliseconds(config.time_to_full_secs * 1000 / 2);

        let pool_msats = 1_000_000_000; // 1M sats = 1B msats
        let result = compute_balance_msats(pool_msats, None, created_at, &config);

        // Expected: pool * 0.1 * 0.5 = 1B * 0.1 * 0.5 = 50M msats = 50k sats
        let expected = (pool_msats as f64 * 0.1 * 0.5) as i64;
        assert!((result - expected).abs() < 1000); // Allow small rounding error
    }

    #[test]
    fn test_full_time_gives_max_fill() {
        let config = test_config();
        let now = Utc::now();
        let created_at = now - Duration::seconds(config.time_to_full_secs);

        let pool_msats = 1_000_000_000; // 1M sats
        let result = compute_balance_msats(pool_msats, None, created_at, &config);

        // Expected: pool * 0.1 = 1B * 0.1 = 100M msats = 100k sats
        let expected = (pool_msats as f64 * config.max_fill_percentage) as i64;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_over_time_caps_at_max_fill() {
        let config = test_config();
        let now = Utc::now();
        let created_at = now - Duration::seconds(config.time_to_full_secs * 2); // Double the time

        let pool_msats = 1_000_000_000;
        let result = compute_balance_msats(pool_msats, None, created_at, &config);

        // Should cap at max_fill, not exceed it
        let expected = (pool_msats as f64 * config.max_fill_percentage) as i64;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_withdrawal_resets_fill() {
        let config = test_config();
        let now = Utc::now();
        let created_at = now - Duration::days(30); // Created 30 days ago
        let last_withdraw_at = Some(now); // Just withdrawn

        let pool_msats = 1_000_000_000;
        let result = compute_balance_msats(pool_msats, last_withdraw_at, created_at, &config);

        // Just withdrawn, should be ~0
        assert_eq!(result, 0);
    }

    #[test]
    fn test_partial_refill_after_withdrawal() {
        let config = test_config();
        let now = Utc::now();
        let created_at = now - Duration::days(30);
        let last_withdraw_at = Some(now - Duration::days(7)); // Withdrew 7 days ago

        let pool_msats = 1_000_000_000;
        let result = compute_balance_msats(pool_msats, last_withdraw_at, created_at, &config);

        // 7/21 = 1/3 of the way to full
        let expected = (pool_msats as f64 * 0.1 * (7.0 / 21.0)) as i64;
        assert!((result - expected).abs() < 1000);
    }

    #[test]
    fn test_different_fill_percentage() {
        let config = BalanceConfig {
            time_to_full_secs: 21 * SECS_PER_DAY,
            max_fill_percentage: 0.05, // 5%
        };
        let now = Utc::now();
        let created_at = now - Duration::days(21);

        let pool_msats = 1_000_000_000;
        let result = compute_balance_msats(pool_msats, None, created_at, &config);

        // Expected: pool * 0.05 = 50M msats
        let expected = (pool_msats as f64 * 0.05) as i64;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_different_time_to_full() {
        let config = BalanceConfig {
            time_to_full_secs: 7 * SECS_PER_DAY, // 1 week
            max_fill_percentage: 0.1,
        };
        let now = Utc::now();
        let created_at = now - Duration::days(7);

        let pool_msats = 1_000_000_000;
        let result = compute_balance_msats(pool_msats, None, created_at, &config);

        // Should be at max after 7 days
        let expected = (pool_msats as f64 * 0.1) as i64;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_sub_day_time_to_full() {
        // 1 hour to full
        let config = BalanceConfig {
            time_to_full_secs: 60 * 60,
            max_fill_percentage: 0.1,
        };
        let now = Utc::now();
        let created_at = now - Duration::seconds(60 * 60); // 1 hour ago

        let pool_msats = 1_000_000_000;
        let result = compute_balance_msats(pool_msats, None, created_at, &config);

        let expected = (pool_msats as f64 * 0.1) as i64;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_with_overrides_falls_back_to_defaults() {
        let base = BalanceConfig::default();
        let overridden = base.with_overrides(None, None);
        assert_eq!(overridden.time_to_full_secs, base.time_to_full_secs);
        assert_eq!(overridden.max_fill_percentage, base.max_fill_percentage);
    }

    #[test]
    fn test_with_overrides_applies_per_location_values() {
        let base = BalanceConfig::default();
        let overridden = base.with_overrides(Some(3600), Some(0.25));
        assert_eq!(overridden.time_to_full_secs, 3600);
        assert_eq!(overridden.max_fill_percentage, 0.25);
    }
}
