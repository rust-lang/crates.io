ALTER TABLE users RENAME TO users_v2;

CREATE VIEW users AS
  SELECT *, login AS gh_login FROM users_v2;
