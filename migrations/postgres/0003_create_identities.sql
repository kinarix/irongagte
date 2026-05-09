CREATE TABLE identities (
    id                UUID        PRIMARY KEY,
    user_id           UUID        NOT NULL REFERENCES users (id),
    tenant_id         UUID        NOT NULL REFERENCES tenants (id),
    provider          TEXT        NOT NULL,
    provider_user_id  TEXT        NOT NULL,
    email             TEXT        NOT NULL,
    raw_claims        JSONB       NOT NULL DEFAULT '{}',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_identities_provider_uid
    ON identities (tenant_id, provider, provider_user_id);

CREATE INDEX idx_identities_user
    ON identities (user_id, tenant_id);
