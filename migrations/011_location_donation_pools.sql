-- Unified donations table replacing donation_pool and pending_donations
-- Status: 'created' (invoice generated), 'received' (paid), 'timed_out' (expired)
-- location_id: NULL for global donations (which get split among all locations at receive time)

CREATE TABLE donations (
    id TEXT PRIMARY KEY,
    location_id TEXT REFERENCES locations(id) ON DELETE SET NULL,
    invoice TEXT NOT NULL,
    amount_msats INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'created', -- 'created', 'received', 'timed_out'
    created_at TIMESTAMP NOT NULL,
    received_at TIMESTAMP
);

CREATE INDEX idx_donations_status ON donations(status);
CREATE INDEX idx_donations_location ON donations(location_id);
CREATE INDEX idx_donations_invoice ON donations(invoice);

-- Track debits from location donation pools (when refills use location pool funds)
CREATE TABLE location_pool_debits (
    id TEXT PRIMARY KEY,
    location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
    amount_msats INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_location_pool_debits_location ON location_pool_debits(location_id);
