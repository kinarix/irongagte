CREATE TABLE identities (
    id                TEXT    PRIMARY KEY,
    user_id           TEXT    NOT NULL REFERENCES users (id),
    tenant_id         TEXT    NOT NULL REFERENCES tenants (id),
    provider          TEXT    NOT NULL,
    provider_user_id  TEXT    NOT NULL,
    email             TEXT    NOT NULL,
    raw_claims        TEXT    NOT NULL DEFAULT '{}',
    created_at        TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at        TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX idx_identities_provider_uid
    ON identities (tenant_id, provider, provider_user_id);

CREATE INDEX idx_identities_user
    ON identities (user_id, tenant_id);
