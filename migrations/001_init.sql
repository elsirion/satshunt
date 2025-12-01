-- Create locations table
CREATE TABLE IF NOT EXISTS locations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    latitude REAL NOT NULL,
    longitude REAL NOT NULL,
    description TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Lightning/sats configuration
    current_sats INTEGER NOT NULL DEFAULT 0,
    max_sats INTEGER NOT NULL,
    lnurlw_secret TEXT NOT NULL UNIQUE,

    -- Last refill tracking
    last_refill_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- One-time write token for NFC setup
    write_token TEXT UNIQUE,
    write_token_used BOOLEAN NOT NULL DEFAULT 0,
    write_token_created_at TIMESTAMP
);

-- Create photos table
CREATE TABLE IF NOT EXISTS photos (
    id TEXT PRIMARY KEY,
    location_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    uploaded_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (location_id) REFERENCES locations(id) ON DELETE CASCADE
);

-- Create donation pool table
CREATE TABLE IF NOT EXISTS donation_pool (
    id INTEGER PRIMARY KEY CHECK (id = 1), -- Singleton table
    total_sats INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Initialize donation pool
INSERT INTO donation_pool (id, total_sats) VALUES (1, 0);

-- Create scans table to track usage
CREATE TABLE IF NOT EXISTS scans (
    id TEXT PRIMARY KEY,
    location_id TEXT NOT NULL,
    sats_withdrawn INTEGER NOT NULL,
    scanned_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (location_id) REFERENCES locations(id) ON DELETE CASCADE
);

-- Create indexes
CREATE INDEX idx_locations_coords ON locations(latitude, longitude);
CREATE INDEX idx_photos_location ON photos(location_id);
CREATE INDEX idx_scans_location ON scans(location_id);
CREATE INDEX idx_scans_time ON scans(scanned_at);
