CREATE TABLE operator_credentials (
    operator_id     UUID        PRIMARY KEY REFERENCES operators (id) ON DELETE CASCADE,
    password_hash   TEXT        NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
