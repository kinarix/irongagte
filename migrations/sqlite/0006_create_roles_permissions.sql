CREATE TABLE roles (
    id              TEXT    PRIMARY KEY,
    tenant_id       TEXT    NOT NULL REFERENCES tenants (id),
    name            TEXT    NOT NULL,
    description     TEXT,
    parent_role_id  TEXT    REFERENCES roles (id),
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX idx_roles_tenant_name
    ON roles (tenant_id, name);

CREATE TABLE permissions (
    id          TEXT    PRIMARY KEY,
    tenant_id   TEXT    NOT NULL REFERENCES tenants (id),
    resource    TEXT    NOT NULL,
    action      TEXT    NOT NULL,
    description TEXT,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX idx_permissions_tenant_resource_action
    ON permissions (tenant_id, resource, action);

CREATE TABLE role_permissions (
    role_id        TEXT NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    permission_id  TEXT NOT NULL REFERENCES permissions (id) ON DELETE CASCADE,
    tenant_id      TEXT NOT NULL,
    PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE user_roles (
    user_id    TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role_id    TEXT NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    tenant_id  TEXT NOT NULL,
    PRIMARY KEY (user_id, role_id)
);

CREATE INDEX idx_user_roles_tenant
    ON user_roles (tenant_id, user_id);
