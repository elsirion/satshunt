-- User transactions table for tracking sat collections and withdrawals
-- This enables the custodial wallet system where users collect sats into their balance

-- First: Modify users table to support anonymous users (username becomes nullable)
-- SQLite requires table recreation to change column constraints
-- We do this FIRST because other tables will reference it
CREATE TABLE users_new (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE,  -- Now nullable for anonymous users (UNIQUE allows multiple NULLs in SQLite)
    email TEXT,
    auth_method TEXT NOT NULL,
    auth_data TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_login_at TIMESTAMP
);

-- Copy existing data
INSERT INTO users_new (id, username, email, auth_method, auth_data, created_at, last_login_at)
SELECT id, username, email, auth_method, auth_data, created_at, last_login_at FROM users;

-- Drop old table and rename new one
DROP TABLE users;
ALTER TABLE users_new RENAME TO users;

-- Recreate index on username
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- Now create the user_transactions table
CREATE TABLE IF NOT EXISTS user_transactions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    location_id TEXT,  -- NULL for withdrawals, set for collections
    msats INTEGER NOT NULL,
    transaction_type TEXT NOT NULL CHECK (transaction_type IN ('collect', 'withdraw')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for efficient balance calculation and history queries
CREATE INDEX IF NOT EXISTS idx_user_transactions_user ON user_transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_user_transactions_user_type ON user_transactions(user_id, transaction_type);
CREATE INDEX IF NOT EXISTS idx_user_transactions_time ON user_transactions(created_at);

-- Add user_id to scans table for linking scans to collectors
-- Note: No FK constraint to avoid issues with existing data and SQLite limitations
ALTER TABLE scans ADD COLUMN user_id TEXT;
CREATE INDEX IF NOT EXISTS idx_scans_user ON scans(user_id);
