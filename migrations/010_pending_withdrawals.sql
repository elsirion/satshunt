-- Pending withdrawals table to track in-flight withdrawals and prevent double-spending
-- Withdrawals are marked pending before payment attempt, then completed or failed after

CREATE TABLE IF NOT EXISTS pending_withdrawals (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    msats INTEGER NOT NULL,
    invoice TEXT NOT NULL,  -- BOLT11 invoice being paid, for later verification with Blitzi
    status TEXT NOT NULL CHECK (status IN ('pending', 'completed', 'failed')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP
);

-- Index for efficient pending withdrawal lookups per user
CREATE INDEX IF NOT EXISTS idx_pending_withdrawals_user_status ON pending_withdrawals(user_id, status);
