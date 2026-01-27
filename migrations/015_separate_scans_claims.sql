-- Separate scanning (NFC tap validation) from claiming (crediting sats)
-- SQLite doesn't support RENAME COLUMN, so we recreate tables

-- Step 1: Create new claims table (replaces old scans table)
CREATE TABLE claims (
    id TEXT PRIMARY KEY,
    location_id TEXT NOT NULL,
    msats_claimed INTEGER NOT NULL,
    claimed_at TIMESTAMP NOT NULL,
    user_id TEXT,
    FOREIGN KEY (location_id) REFERENCES locations(id) ON DELETE CASCADE
);

-- Step 2: Copy data from scans to claims
INSERT INTO claims (id, location_id, msats_claimed, claimed_at, user_id)
SELECT id, location_id, msats_withdrawn, scanned_at, user_id FROM scans;

-- Step 3: Drop old scans table and its indexes
DROP INDEX IF EXISTS idx_scans_location;
DROP INDEX IF EXISTS idx_scans_user;
DROP INDEX IF EXISTS idx_scans_time;
DROP TABLE scans;

-- Step 4: Create indexes on claims
CREATE INDEX idx_claims_location ON claims(location_id);
CREATE INDEX idx_claims_user ON claims(user_id);
CREATE INDEX idx_claims_time ON claims(claimed_at);

-- Step 5: Create new scans table (records NFC taps before claiming)
CREATE TABLE scans (
    id TEXT PRIMARY KEY,
    location_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    counter INTEGER NOT NULL,
    scanned_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    claimed_at TIMESTAMP,
    claim_id TEXT,
    FOREIGN KEY (location_id) REFERENCES locations(id) ON DELETE CASCADE
);

CREATE INDEX idx_scans_location_time ON scans(location_id, scanned_at DESC);
CREATE INDEX idx_scans_user ON scans(user_id);
