CREATE TABLE reserved_usernames (
    username TEXT PRIMARY KEY
);
CREATE UNIQUE INDEX ON reserved_usernames (lower(username));

CREATE FUNCTION ensure_username_not_reserved() RETURNS trigger AS $$
BEGIN
    IF lower(NEW.username) IN (
        SELECT lower(username) FROM reserved_usernames
    ) THEN
        RAISE EXCEPTION 'cannot create user with reserved username';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_ensure_username_not_reserved
BEFORE INSERT OR UPDATE ON users
FOR EACH ROW EXECUTE PROCEDURE ensure_username_not_reserved();
