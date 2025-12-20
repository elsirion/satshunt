-- Add last_withdraw_at column to track when a location was last withdrawn from
-- Used to calculate refill delta: we use the smaller of (now - last_refill_at) and (now - last_withdraw_at)
ALTER TABLE locations ADD COLUMN last_withdraw_at TIMESTAMP;
