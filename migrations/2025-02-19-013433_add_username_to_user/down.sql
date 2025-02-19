DROP INDEX IF EXISTS lower_username;

ALTER TABLE users
DROP COLUMN username;
