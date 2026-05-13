CREATE TABLE idp_configs (
    id             UUID        PRIMARY KEY,
    tenant_id      UUID        NOT NULL REFERENCES tenants (id),
    provider_type  TEXT        NOT NULL,
    name           TEXT        NOT NULL,
    enabled        BOOLEAN     NOT NULL DEFAULT true,
    config         JSONB       NOT NULL DEFAULT '{}',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_idp_configs_tenant
    ON idp_configs (tenant_id)
    WHERE enabled = true;
