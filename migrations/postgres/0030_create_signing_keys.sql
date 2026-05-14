CREATE TABLE signing_keys (
    id              UUID PRIMARY KEY,
    tenant_id       UUID NULL REFERENCES tenants(id) ON DELETE CASCADE,
    algorithm       TEXT NOT NULL,
    private_key_pem TEXT NOT NULL,
    public_key_pem  TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL,
    retired_at      TIMESTAMPTZ NULL
);

CREATE INDEX idx_signing_keys_active
    ON signing_keys (tenant_id, retired_at, expires_at);

CREATE INDEX idx_signing_keys_current
    ON signing_keys (tenant_id, created_at DESC)
    WHERE retired_at IS NULL;
