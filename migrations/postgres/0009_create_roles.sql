CREATE TABLE roles (
    id              UUID        PRIMARY KEY,
    tenant_id       UUID        NOT NULL REFERENCES tenants (id),
    name            TEXT        NOT NULL,
    description     TEXT,
    parent_role_id  UUID        REFERENCES roles (id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_roles_tenant_name
    ON roles (tenant_id, name);
