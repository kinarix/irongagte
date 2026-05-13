-- Join table: which roles is each user assigned?
CREATE TABLE user_roles (
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role_id    UUID NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    tenant_id  UUID NOT NULL,
    PRIMARY KEY (user_id, role_id)
);

CREATE INDEX idx_user_roles_tenant
    ON user_roles (tenant_id, user_id);
