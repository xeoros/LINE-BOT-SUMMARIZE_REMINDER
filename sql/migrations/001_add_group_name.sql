-- Migration: Add group_name column to checklists table
-- This migration adds support for storing LINE group/chat names
-- Version: 001
-- Date: 2026-03-27

-- Add group_name column to checklists table if it doesn't already exist
ALTER TABLE checklists
ADD COLUMN IF NOT EXISTS group_name TEXT;

-- Add comment to document the column
COMMENT ON COLUMN checklists.group_name IS 'Group/chat name from LINE API (fetched via get_group_summary endpoint)';

-- Create an index on group_name for faster lookups
CREATE INDEX IF NOT EXISTS idx_checklists_group_name ON checklists(group_name);

-- Verify the migration by checking if column exists
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_name = 'checklists'
        AND column_name = 'group_name'
    ) THEN
        RAISE NOTICE 'Column group_name successfully added to checklists table';
    ELSE
        RAISE EXCEPTION 'Failed to add group_name column to checklists table';
    END IF;
END $$;
