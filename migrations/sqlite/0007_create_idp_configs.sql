CREATE TABLE idp_configs (
    id             TEXT    PRIMARY KEY,
    tenant_id      TEXT    NOT NULL REFERENCES tenants (id),
    provider_type  TEXT    NOT NULL,
    name           TEXT    NOT NULL,
    enabled        INTEGER NOT NULL DEFAULT 1,
    config         TEXT    NOT NULL DEFAULT '{}',
    created_at     TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at     TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_idp_configs_tenant
    ON idp_configs (tenant_id)
    WHERE enabled = 1;
