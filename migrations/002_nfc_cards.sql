-- Create NFC cards table to store boltcard keys
CREATE TABLE IF NOT EXISTS nfc_cards (
    id TEXT PRIMARY KEY,
    location_id TEXT NOT NULL UNIQUE,

    -- Boltcard keys
    k0_auth_key TEXT NOT NULL,      -- Authentication key
    k1_decrypt_key TEXT NOT NULL,   -- Decryption key (shared across all cards)
    k2_cmac_key TEXT NOT NULL,      -- CMAC key for verification
    k3 TEXT NOT NULL,                -- Additional key 3
    k4 TEXT NOT NULL,                -- Additional key 4

    -- Card state
    uid TEXT,                        -- Card UID (set after first program)
    counter INTEGER NOT NULL DEFAULT 0,  -- Replay protection counter
    version INTEGER NOT NULL DEFAULT 0,  -- Key version for deterministic key gen

    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    programmed_at TIMESTAMP,         -- When card was first programmed
    last_used_at TIMESTAMP,          -- Last successful tap

    FOREIGN KEY (location_id) REFERENCES locations(id) ON DELETE CASCADE
);

-- Create index for fast UID lookups during payment verification
CREATE INDEX idx_nfc_cards_uid ON nfc_cards(uid) WHERE uid IS NOT NULL;
CREATE INDEX idx_nfc_cards_location ON nfc_cards(location_id);
