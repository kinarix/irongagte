CREATE TABLE group_members (
    group_id   UUID NOT NULL REFERENCES groups (id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    tenant_id  UUID NOT NULL,
    PRIMARY KEY (group_id, user_id)
);

CREATE INDEX idx_group_members_user ON group_members (user_id, tenant_id);
