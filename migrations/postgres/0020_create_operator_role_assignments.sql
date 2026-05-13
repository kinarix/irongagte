CREATE TABLE operator_role_assignments (
    operator_id       UUID NOT NULL REFERENCES operators (id) ON DELETE CASCADE,
    operator_role_id  UUID NOT NULL REFERENCES operator_roles (id) ON DELETE CASCADE,
    assigned_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (operator_id, operator_role_id)
);

CREATE INDEX idx_operator_role_assignments_role
    ON operator_role_assignments (operator_role_id);
