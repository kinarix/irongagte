CREATE TABLE users (
    id              UUID        PRIMARY KEY,
    tenant_id       UUID        NOT NULL REFERENCES tenants (id),
    email           TEXT        NOT NULL,
    email_verified  BOOLEAN     NOT NULL DEFAULT false,
    name            TEXT,
    given_name      TEXT,
    family_name     TEXT,
    picture_url     TEXT,
    status          TEXT        NOT NULL DEFAULT 'pending',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at   TIMESTAMPTZ,
    deleted_at      TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_users_tenant_email_active
    ON users (tenant_id, email)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_users_tenant_status
    ON users (tenant_id, status)
    WHERE deleted_at IS NULL;
