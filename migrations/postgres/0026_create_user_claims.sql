-- Claim values assigned directly to a user. For scalar claims, a user-direct
-- value overrides any group-derived value. For multi claims, user-direct
-- values merge with group-derived values (deduped).
CREATE TABLE user_claims (
    user_id         UUID        NOT NULL REFERENCES users (id)              ON DELETE CASCADE,
    claim_def_id    UUID        NOT NULL REFERENCES claim_definitions (id)  ON DELETE CASCADE,
    value           TEXT        NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, claim_def_id, value)
);

CREATE INDEX idx_user_claims_claim_def
    ON user_claims (claim_def_id);
