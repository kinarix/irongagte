ALTER TABLE users
    ADD COLUMN attributes JSONB NOT NULL DEFAULT '{}'::jsonb;
