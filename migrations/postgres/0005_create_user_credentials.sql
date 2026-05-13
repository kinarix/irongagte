CREATE TABLE user_credentials (
    id            UUID        PRIMARY KEY,
    tenant_id     UUID        NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    user_id       UUID        NOT NULL UNIQUE REFERENCES users (id) ON DELETE CASCADE,
    password_hash TEXT,
    totp_secret   TEXT,
    totp_enabled  BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_credentials_tenant_id ON user_credentials (tenant_id);
