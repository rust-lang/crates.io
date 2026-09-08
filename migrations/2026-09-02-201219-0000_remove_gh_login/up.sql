DROP INDEX IF EXISTS lower_gh_login;
-- safety-assured:start
-- The column is no longer read or written. GitHub logins are stored in
-- `oauth_github.login`.
ALTER TABLE users DROP COLUMN gh_login;
-- safety-assured:end
