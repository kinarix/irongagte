CREATE TABLE tenants (
    id          TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL,
    slug        TEXT    NOT NULL,
    settings    TEXT    NOT NULL DEFAULT '{}',
    created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    deleted_at  TEXT
);

CREATE UNIQUE INDEX idx_tenants_slug_active
    ON tenants (slug)
    WHERE deleted_at IS NULL;
