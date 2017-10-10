DROP INDEX IF EXISTS index_users_gh_login;
CREATE INDEX lower_gh_login ON users (lower(gh_login));
