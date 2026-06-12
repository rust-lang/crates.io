DROP TRIGGER IF EXISTS trigger_ensure_username_not_reserved ON users;
DROP FUNCTION IF EXISTS ensure_username_not_reserved();
-- safety-assured:start
DROP TABLE IF EXISTS reserved_usernames;
-- safety-assured:end
DROP FUNCTION IF EXISTS canon_username(text);
