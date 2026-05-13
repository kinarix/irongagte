-- OAuth clients (RPs). `claims_config` is per-app JSON describing which
-- additional top-level claims to merge into access tokens minted for this app.
CREATE TABLE applications (
    id                  UUID        PRIMARY KEY,
    tenant_id           UUID        NOT NULL REFERENCES tenants (id),
    name                TEXT        NOT NULL,
    client_id           TEXT        NOT NULL,
    client_secret_hash  TEXT,
    app_type            TEXT        NOT NULL,
    redirect_uris       TEXT        NOT NULL DEFAULT '[]',
    allowed_scopes      TEXT        NOT NULL DEFAULT '[]',
    grant_types         TEXT        NOT NULL DEFAULT '[]',
    access_token_ttl    BIGINT      NOT NULL DEFAULT 3600,
    refresh_token_ttl   BIGINT      NOT NULL DEFAULT 2592000,
    claims_config       TEXT        NOT NULL DEFAULT '{"mappers":[]}',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at          TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_applications_client_id_active
    ON applications (tenant_id, client_id)
    WHERE deleted_at IS NULL;
