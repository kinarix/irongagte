-- Operators: irongate dashboard administrators. No tenant scope — these manage
-- the IAM instance itself, not the end users authenticating *through* it.
CREATE TABLE operators (
    id              UUID        PRIMARY KEY,
    email           TEXT        NOT NULL,
    name            TEXT,
    status          TEXT        NOT NULL DEFAULT 'active',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at   TIMESTAMPTZ,
    deleted_at      TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_operators_email_active
    ON operators (email)
    WHERE deleted_at IS NULL;
