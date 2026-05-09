CREATE TABLE audit_events (
    id          TEXT    PRIMARY KEY,
    tenant_id   TEXT    NOT NULL,
    event_type  TEXT    NOT NULL,
    actor_id    TEXT,
    target_id   TEXT,
    ip_address  TEXT,
    metadata    TEXT    NOT NULL DEFAULT '{}',
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_audit_events_tenant_created
    ON audit_events (tenant_id, created_at DESC);

CREATE INDEX idx_audit_events_actor
    ON audit_events (actor_id)
    WHERE actor_id IS NOT NULL;
