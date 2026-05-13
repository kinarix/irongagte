-- Each application owns a namespace prefix for its custom JWT claims.
-- Custom claims emitted into a token issued for this app are keyed as
-- `<claim_prefix>:<claim_key>` (e.g. `billing:plan`). Standard OIDC claims
-- (`sub`, `email`, `name`, ...) remain unprefixed.
ALTER TABLE applications
    ADD COLUMN claim_prefix TEXT NOT NULL DEFAULT '';

-- Backfill existing rows with a slug-like value derived from client_id so the
-- unique index below doesn't collide. Operators will rename as needed.
UPDATE applications
SET claim_prefix = regexp_replace(lower(client_id), '[^a-z0-9_-]+', '-', 'g')
WHERE claim_prefix = '';

-- Prefix uniqueness within a tenant. Soft-deleted apps are excluded.
CREATE UNIQUE INDEX idx_applications_claim_prefix_active
    ON applications (tenant_id, claim_prefix)
    WHERE deleted_at IS NULL;

-- Drop the now-defunct claims_config column. The mapper-based projection
-- system is replaced by the (claim_definitions, group_claims, user_claims)
-- trio introduced in migrations 0024-0026.
ALTER TABLE applications
    DROP COLUMN claims_config;
