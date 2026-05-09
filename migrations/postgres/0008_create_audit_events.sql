CREATE TABLE audit_events (
    id          UUID        PRIMARY KEY,
    tenant_id   UUID        NOT NULL,
    event_type  TEXT        NOT NULL,
    actor_id    UUID,
    target_id   UUID,
    ip_address  TEXT,
    metadata    JSONB       NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_events_tenant_created
    ON audit_events (tenant_id, created_at DESC);

CREATE INDEX idx_audit_events_actor
    ON audit_events (actor_id)
    WHERE actor_id IS NOT NULL;
