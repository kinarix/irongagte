-- System-level events (operator, operator-role, operator-permission CRUD) have
-- no tenant context. Relax the NOT NULL constraint so they can be recorded with
-- tenant_id IS NULL. The (tenant_id, created_at) index continues to work — Postgres
-- B-tree indexes include NULLs.
ALTER TABLE audit_events ALTER COLUMN tenant_id DROP NOT NULL;
