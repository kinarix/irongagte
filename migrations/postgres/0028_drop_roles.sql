-- `Role` is no longer a first-class entity. Roles are expressed as a regular
-- multi-typed claim (e.g. an app with claim_prefix='billing' defines a claim
-- named 'roles'; groups carry the role values; members inherit them). All
-- former role data is dropped — the codebase is pre-production.
DROP TABLE IF EXISTS user_roles;
DROP TABLE IF EXISTS roles;
