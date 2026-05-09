CREATE TABLE users (
    id              TEXT    PRIMARY KEY,
    tenant_id       TEXT    NOT NULL REFERENCES tenants (id),
    email           TEXT    NOT NULL,
    email_verified  INTEGER NOT NULL DEFAULT 0,
    name            TEXT,
    given_name      TEXT,
    family_name     TEXT,
    picture_url     TEXT,
    status          TEXT    NOT NULL DEFAULT 'pending',
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    last_login_at   TEXT,
    deleted_at      TEXT
);

CREATE UNIQUE INDEX idx_users_tenant_email_active
    ON users (tenant_id, email)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_users_tenant_status
    ON users (tenant_id, status)
    WHERE deleted_at IS NULL;
