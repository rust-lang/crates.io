-- This migration was merged into the master branch but could not be deployed to production
-- because production gh_login isn't actually unique. Later, this migration was commented out
-- so that it will be a no-op on production, and a new migration was added to correct the database
-- of anyone who ran this migration locally.
--
-- DROP INDEX IF EXISTS index_users_gh_login;
-- ALTER TABLE users ADD CONSTRAINT unique_gh_login UNIQUE(gh_login);
-- CREATE UNIQUE INDEX lower_gh_login ON users (lower(gh_login));
SELECT 1;