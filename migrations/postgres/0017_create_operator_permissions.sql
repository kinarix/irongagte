CREATE TABLE operator_permissions (
    id          UUID        PRIMARY KEY,
    resource    TEXT        NOT NULL,
    action      TEXT        NOT NULL,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (resource, action)
);

CREATE INDEX idx_operator_permissions_resource
    ON operator_permissions (resource);
