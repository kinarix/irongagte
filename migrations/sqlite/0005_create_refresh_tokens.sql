-- session_id references the Redis session key (not a DB table)
CREATE TABLE refresh_tokens (
    id              TEXT    PRIMARY KEY,
    session_id      TEXT    NOT NULL,
    application_id  TEXT    NOT NULL REFERENCES applications (id),
    token_hash      TEXT    NOT NULL UNIQUE,
    scope           TEXT    NOT NULL,
    previous_id     TEXT,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    expires_at      TEXT    NOT NULL,
    revoked_at      TEXT
);

CREATE INDEX idx_refresh_tokens_session
    ON refresh_tokens (session_id);

CREATE INDEX idx_refresh_tokens_expires_active
    ON refresh_tokens (expires_at)
    WHERE revoked_at IS NULL;
