SET LOCAL lock_timeout = '10s';
SET LOCAL statement_timeout = '120s';

-- Record which OAuth provider a user treats as their primary identity.
-- For every existing user this is 'github' (the only login path to date),
-- so NOT NULL DEFAULT 'github' is accurate and avoids a separate backfill.
-- PG 11+ optimizes ADD COLUMN ... NOT NULL DEFAULT <constant> as a
-- metadata-only operation, so this does not rewrite the table.
ALTER TABLE users
  ADD COLUMN primary_oauth_provider oauth_provider NOT NULL DEFAULT 'github';
