-- Pending donations table for resilient donation tracking
-- Donations are stored here when an invoice is created, and completed when payment is received
-- This allows the server to resume tracking donations after restarts and handles
-- cases where the client disconnects before payment is confirmed.

CREATE TABLE pending_donations (
    id TEXT PRIMARY KEY,
    invoice TEXT NOT NULL UNIQUE,
    amount_msats INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'completed', 'expired'
    created_at TIMESTAMP NOT NULL,
    completed_at TIMESTAMP
);

-- Index for efficient lookup of pending donations
CREATE INDEX idx_pending_donations_status ON pending_donations(status);
