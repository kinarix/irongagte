CREATE TABLE passkeys (
    id              UUID        PRIMARY KEY,
    tenant_id       UUID        NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    user_id         UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    credential_id   TEXT        NOT NULL,
    friendly_name   TEXT,
    passkey_json    JSONB       NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at    TIMESTAMPTZ,
    UNIQUE (tenant_id, credential_id)
);

CREATE INDEX idx_passkeys_user_id       ON passkeys (user_id);
CREATE INDEX idx_passkeys_tenant_id     ON passkeys (tenant_id);
CREATE INDEX idx_passkeys_credential_id ON passkeys (credential_id);
