CREATE TABLE magic_links (
    id          UUID        PRIMARY KEY,
    tenant_id   UUID        NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    user_id     UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash  TEXT        NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    used_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_magic_links_user_id    ON magic_links (user_id);
CREATE INDEX idx_magic_links_tenant_id  ON magic_links (tenant_id);
