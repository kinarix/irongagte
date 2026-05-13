CREATE TABLE IF NOT EXISTS passkeys (
    id              TEXT NOT NULL PRIMARY KEY,
    tenant_id       TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id   TEXT NOT NULL,
    friendly_name   TEXT,
    passkey_json    TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    last_used_at    TEXT,
    UNIQUE (tenant_id, credential_id)
);

CREATE INDEX IF NOT EXISTS idx_passkeys_user_id       ON passkeys(user_id);
CREATE INDEX IF NOT EXISTS idx_passkeys_tenant_id     ON passkeys(tenant_id);
CREATE INDEX IF NOT EXISTS idx_passkeys_credential_id ON passkeys(credential_id);
