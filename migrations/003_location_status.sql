-- Add status field to locations table
-- Status can be: 'created', 'programmed', 'active'
ALTER TABLE locations ADD COLUMN status TEXT NOT NULL DEFAULT 'created';

-- Create index for filtering by status
CREATE INDEX idx_locations_status ON locations(status);

-- Update existing locations to 'active' status if they don't have a write_token
-- (these are legacy locations created before this migration)
UPDATE locations SET status = 'active' WHERE write_token IS NULL;
