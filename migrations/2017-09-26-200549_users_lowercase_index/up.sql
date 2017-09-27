DROP INDEX IF EXISTS index_users_gh_login;
ALTER TABLE users ADD CONSTRAINT unique_gh_login UNIQUE(gh_login);
CREATE UNIQUE INDEX lower_gh_login ON users (lower(gh_login));
