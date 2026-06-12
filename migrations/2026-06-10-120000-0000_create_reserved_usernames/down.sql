DROP TRIGGER IF EXISTS trigger_ensure_username_not_reserved ON users;
DROP FUNCTION IF EXISTS ensure_username_not_reserved();
-- safety-assured:start
-- Suppresses the "DROP TABLE" check. This is a down migration that is only run
-- manually during development; the table will be dropped before it contains any
-- significant data.
DROP TABLE IF EXISTS reserved_usernames;
-- safety-assured:end
DROP FUNCTION IF EXISTS canon_username(text);
