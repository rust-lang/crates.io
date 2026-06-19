-- safety-assured:start
-- In production a validated `versions_tar_sha256_present`
-- CHECK (tar_sha256 IS NOT NULL) already proves every row is non-null, so
-- Postgres skips the table scan that `SET NOT NULL` would otherwise perform
-- under ACCESS EXCLUSIVE. That check is added and validated as a manual step
-- on the live database before this migration runs.
ALTER TABLE versions ALTER COLUMN tar_sha256 SET NOT NULL;
-- safety-assured:end

ALTER TABLE versions DROP CONSTRAINT IF EXISTS versions_tar_sha256_present;
