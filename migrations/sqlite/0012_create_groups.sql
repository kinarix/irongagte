CREATE TABLE groups (
    id           TEXT    PRIMARY KEY,
    tenant_id    TEXT    NOT NULL REFERENCES tenants (id),
    display_name TEXT    NOT NULL,
    external_id  TEXT,
    created_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX idx_groups_tenant_name
    ON groups (tenant_id, display_name);

CREATE TABLE group_members (
    group_id   TEXT NOT NULL REFERENCES groups (id) ON DELETE CASCADE,
    user_id    TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    tenant_id  TEXT NOT NULL,
    PRIMARY KEY (group_id, user_id)
);

CREATE INDEX idx_group_members_user ON group_members (user_id, tenant_id);
