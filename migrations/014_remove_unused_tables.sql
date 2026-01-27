-- Migration: Remove unused tables after switching to computed balance system
--
-- The balance is now computed on-demand from:
--   pool_balance = SUM(donations) - SUM(scans)
--   available = pool_balance * max_fill_percentage * fill_ratio
--
-- These tables are no longer written to and can be removed:

-- Drop refills table (periodic refill log - no longer used)
DROP TABLE IF EXISTS refills;

-- Drop location_pool_debits table (pool debit tracking - now using scans instead)
DROP TABLE IF EXISTS location_pool_debits;
