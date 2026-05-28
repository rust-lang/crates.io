-- safety-assured:start
-- The table rename and column rename look dangerous to running application
-- instances, but the `users` view created below re-exposes the renamed
-- column under both its old name (`gh_login`) and its new name (`login`).
-- Code that still references `users.gh_login` keeps working unchanged
-- because PostgreSQL rewrites view reads and writes against the underlying
-- `users_v2.login` column.
ALTER TABLE users RENAME TO users_v2;
ALTER TABLE users_v2 RENAME COLUMN gh_login TO login;
ALTER INDEX lower_gh_login RENAME TO lower_login;

CREATE VIEW users AS
  SELECT *, login AS gh_login FROM users_v2;
-- safety-assured:end
