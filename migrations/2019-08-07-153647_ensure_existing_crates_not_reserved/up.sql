CREATE FUNCTION ensure_reserved_name_not_in_use() RETURNS trigger AS $$
BEGIN
    IF canon_crate_name(NEW.name) IN (
        SELECT canon_crate_name(name) FROM crates
    ) THEN
        RAISE EXCEPTION 'crate exists with name %', NEW.name;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_ensure_reserved_name_not_in_use
BEFORE INSERT OR UPDATE ON reserved_crate_names
FOR EACH ROW EXECUTE PROCEDURE ensure_reserved_name_not_in_use();
