DROP VIEW users;

ALTER INDEX lower_login RENAME TO lower_gh_login;
ALTER TABLE users_v2 RENAME COLUMN login TO gh_login;
ALTER TABLE users_v2 RENAME TO users;
