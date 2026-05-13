-- Scope OperatorRole to a tenant: NULL = global (cross-tenant), non-NULL = scoped
-- to a single tenant. Permissions catalog stays global; only role scope is added.

ALTER TABLE operator_roles
    ADD COLUMN tenant_id UUID REFERENCES tenants (id) ON DELETE CASCADE;

ALTER TABLE operator_roles
    DROP CONSTRAINT operator_roles_name_key;

CREATE UNIQUE INDEX idx_operator_roles_global_name
    ON operator_roles (name)
    WHERE tenant_id IS NULL;

CREATE UNIQUE INDEX idx_operator_roles_tenant_name
    ON operator_roles (tenant_id, name)
    WHERE tenant_id IS NOT NULL;

CREATE INDEX idx_operator_roles_tenant
    ON operator_roles (tenant_id)
    WHERE tenant_id IS NOT NULL;
