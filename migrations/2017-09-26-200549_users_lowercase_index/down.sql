DROP INDEX lower_gh_login;
ALTER TABLE users DROP CONSTRAINT unique_gh_login;
CREATE INDEX index_users_gh_login ON users (gh_login);
