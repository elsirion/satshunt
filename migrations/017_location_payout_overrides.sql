-- Per-location payout schedule overrides.
-- NULL = use the global default from BalanceConfig.
ALTER TABLE locations ADD COLUMN time_to_full_secs INTEGER;
ALTER TABLE locations ADD COLUMN max_fill_percentage REAL;
