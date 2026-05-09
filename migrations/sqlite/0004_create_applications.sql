CREATE TABLE applications (
    id                  TEXT    PRIMARY KEY,
    tenant_id           TEXT    NOT NULL REFERENCES tenants (id),
    name                TEXT    NOT NULL,
    client_id           TEXT    NOT NULL,
    client_secret_hash  TEXT,
    app_type            TEXT    NOT NULL,
    redirect_uris       TEXT    NOT NULL DEFAULT '[]',
    allowed_scopes      TEXT    NOT NULL DEFAULT '[]',
    grant_types         TEXT    NOT NULL DEFAULT '[]',
    access_token_ttl    INTEGER NOT NULL DEFAULT 3600,
    refresh_token_ttl   INTEGER NOT NULL DEFAULT 2592000,
    created_at          TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at          TEXT    NOT NULL DEFAULT (datetime('now')),
    deleted_at          TEXT
);

CREATE UNIQUE INDEX idx_applications_client_id_active
    ON applications (tenant_id, client_id)
    WHERE deleted_at IS NULL;
