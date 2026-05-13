-- session_id references a key in the Redis session store, not a DB table.
CREATE TABLE refresh_tokens (
    id              UUID        PRIMARY KEY,
    session_id      UUID        NOT NULL,
    application_id  UUID        NOT NULL REFERENCES applications (id),
    token_hash      TEXT        NOT NULL UNIQUE,
    scope           TEXT        NOT NULL,
    previous_id     UUID,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL,
    revoked_at      TIMESTAMPTZ
);

CREATE INDEX idx_refresh_tokens_session
    ON refresh_tokens (session_id);

CREATE INDEX idx_refresh_tokens_expires_active
    ON refresh_tokens (expires_at)
    WHERE revoked_at IS NULL;
