-- Backfill scans table from existing claims
-- This creates historical scan records for claims that have user_id set
-- Counter is set to 0 for historical records (actual counter values not available)

INSERT INTO scans (id, location_id, user_id, counter, scanned_at, claimed_at, claim_id)
SELECT
    'scan-' || id,  -- Generate a new scan ID based on claim ID
    location_id,
    user_id,
    0,              -- Counter not available for historical records
    claimed_at,     -- Use claimed_at as scanned_at (they were the same in old flow)
    claimed_at,     -- Mark as claimed
    id              -- Link to the claim
FROM claims
WHERE user_id IS NOT NULL;
