create function set_updated_at_ignore_downloads() returns trigger
    language plpgsql
as
$$
DECLARE
    new_downloads integer;
BEGIN
    new_downloads := NEW.downloads;
    OLD.downloads := NEW.downloads;
    IF (
        NEW IS DISTINCT FROM OLD AND
        NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at
        ) THEN
        NEW.updated_at = CURRENT_TIMESTAMP;
    END IF;
    NEW.downloads := new_downloads;
    RETURN NEW;
END
$$;
