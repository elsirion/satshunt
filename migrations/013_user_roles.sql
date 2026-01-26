-- Add role column to users table
-- Roles: 'user' (default), 'creator', 'admin'
-- - user: basic access, can collect sats
-- - creator: can create locations
-- - admin: full access, can manage users and roles
ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'user';

-- Create index for role queries
CREATE INDEX idx_users_role ON users(role);
