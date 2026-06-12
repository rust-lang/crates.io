CREATE FUNCTION canon_username(text) RETURNS text AS $$
    SELECT replace(lower($1), '-', '_');
$$ LANGUAGE SQL IMMUTABLE;

-- safety-assured:start
-- Suppresses the "CREATE TABLE without IF NOT EXISTS" diesel-guard check.
-- The table is brand new and `IF NOT EXISTS` is already used, so retrying a
-- partially-failed migration is safe.
CREATE TABLE IF NOT EXISTS reserved_usernames (
    username TEXT PRIMARY KEY
);
-- safety-assured:end

-- safety-assured:start
-- Suppresses the "ADD INDEX without CONCURRENTLY" and "CREATE INDEX without
-- IF NOT EXISTS" diesel-guard checks. The table is brand new and empty, so
-- the unique index builds instantly with no meaningful SHARE lock contention;
-- CONCURRENTLY is unnecessary. `IF NOT EXISTS` is already used for idempotent
-- retries.
CREATE UNIQUE INDEX IF NOT EXISTS idx_reserved_usernames_canon_username ON reserved_usernames (canon_username(username));
-- safety-assured:end

CREATE FUNCTION ensure_username_not_reserved() RETURNS trigger AS $$
BEGIN
    IF canon_username(NEW.username) IN (
        SELECT canon_username(username) FROM reserved_usernames
    ) THEN
        RAISE EXCEPTION 'cannot create user with reserved username';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_ensure_username_not_reserved
BEFORE INSERT OR UPDATE ON users
FOR EACH ROW EXECUTE PROCEDURE ensure_username_not_reserved();
