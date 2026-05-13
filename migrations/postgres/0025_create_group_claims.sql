-- Claim values assigned to a group. Members inherit these at token-mint time.
-- For multi-typed claims a group may have multiple rows for the same claim_def
-- (each row contributing one entry to the resulting array). Uniqueness is on
-- (group_id, claim_def_id, value) so the same value isn't duplicated.
CREATE TABLE group_claims (
    group_id        UUID        NOT NULL REFERENCES groups (id)             ON DELETE CASCADE,
    claim_def_id    UUID        NOT NULL REFERENCES claim_definitions (id)  ON DELETE CASCADE,
    value           TEXT        NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (group_id, claim_def_id, value)
);

CREATE INDEX idx_group_claims_claim_def
    ON group_claims (claim_def_id);
