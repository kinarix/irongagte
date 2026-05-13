CREATE TABLE operator_role_permissions (
    operator_role_id       UUID NOT NULL REFERENCES operator_roles (id) ON DELETE CASCADE,
    operator_permission_id UUID NOT NULL REFERENCES operator_permissions (id) ON DELETE CASCADE,
    PRIMARY KEY (operator_role_id, operator_permission_id)
);

CREATE INDEX idx_operator_role_perms_permission
    ON operator_role_permissions (operator_permission_id);
