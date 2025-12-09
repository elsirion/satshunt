-- Create refills table to track all refill operations
CREATE TABLE IF NOT EXISTS refills (
    id TEXT PRIMARY KEY,
    location_id TEXT NOT NULL,
    msats_added INTEGER NOT NULL,
    balance_before_msats INTEGER NOT NULL,
    balance_after_msats INTEGER NOT NULL,
    base_rate_msats_per_min INTEGER NOT NULL,
    slowdown_factor REAL NOT NULL,
    refilled_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (location_id) REFERENCES locations(id) ON DELETE CASCADE
);

-- Create index for efficient queries by location
CREATE INDEX idx_refills_location ON refills(location_id, refilled_at DESC);
