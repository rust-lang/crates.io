DROP INDEX IF EXISTS lower_gh_login;
CREATE INDEX index_users_gh_login ON users (gh_login);
