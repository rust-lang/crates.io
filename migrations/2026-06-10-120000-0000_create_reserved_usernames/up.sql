CREATE FUNCTION canon_username(text) RETURNS text AS $$
    SELECT replace(lower($1), '-', '_');
$$ LANGUAGE SQL IMMUTABLE;

-- safety-assured:start
CREATE TABLE IF NOT EXISTS reserved_usernames (
    username TEXT PRIMARY KEY
);
-- safety-assured:end

-- safety-assured:start
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
