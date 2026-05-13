-- A claim definition declares one custom JWT claim that an application emits.
-- `key` is unprefixed (e.g. `plan`, `roles`); at token-mint time the actual
-- JWT field becomes `<application.claim_prefix>:<key>`.
--
-- `type` is one of:
--   'scalar' — single string value. Conflicts resolve by precedence (user
--              direct > group with highest priority > created_at asc).
--   'multi'  — list of string values. Conflicts merge into a deduped array.
CREATE TABLE claim_definitions (
    id              UUID        PRIMARY KEY,
    application_id  UUID        NOT NULL REFERENCES applications (id) ON DELETE CASCADE,
    key             TEXT        NOT NULL,
    claim_type      TEXT        NOT NULL CHECK (claim_type IN ('scalar', 'multi')),
    description     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_claim_definitions_app_key
    ON claim_definitions (application_id, key);
