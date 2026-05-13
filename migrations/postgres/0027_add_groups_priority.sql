-- Used to deterministically resolve scalar claim conflicts when a user belongs
-- to multiple groups that assign the same scalar claim. Higher wins; ties
-- broken by created_at ascending.
ALTER TABLE groups
    ADD COLUMN priority INTEGER NOT NULL DEFAULT 0;
