-- Convert all sats columns to msats for accurate accounting
-- This allows us to track routing fees and smaller amounts precisely

-- Update locations table: current_sats -> current_msats
ALTER TABLE locations RENAME COLUMN current_sats TO current_msats;
UPDATE locations SET current_msats = current_msats * 1000;

-- Update donation_pool table: total_sats -> total_msats
ALTER TABLE donation_pool RENAME COLUMN total_sats TO total_msats;
UPDATE donation_pool SET total_msats = total_msats * 1000;

-- Update scans table: sats_withdrawn -> msats_withdrawn
ALTER TABLE scans RENAME COLUMN sats_withdrawn TO msats_withdrawn;
UPDATE scans SET msats_withdrawn = msats_withdrawn * 1000;
