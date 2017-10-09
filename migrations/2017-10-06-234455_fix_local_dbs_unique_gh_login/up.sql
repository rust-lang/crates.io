-- This migration should have no effect on production.
-- Its sole purpose is to fix the local database of anyone who ran 2017-09-26-200549 locally--
-- that migration can't be run on production because lower(gh_login) isn't unique on production.

CREATE INDEX IF NOT EXISTS index_users_gh_login ON users (gh_login);
ALTER TABLE users DROP CONSTRAINT IF EXISTS unique_gh_login;
DROP INDEX IF EXISTS lower_gh_login;