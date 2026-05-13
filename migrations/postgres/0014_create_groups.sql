CREATE TABLE groups (
    id           UUID        PRIMARY KEY,
    tenant_id    UUID        NOT NULL REFERENCES tenants (id),
    display_name TEXT        NOT NULL,
    external_id  TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_groups_tenant_name
    ON groups (tenant_id, display_name);
