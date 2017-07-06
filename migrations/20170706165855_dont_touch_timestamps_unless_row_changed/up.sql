CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger AS $$
BEGIN
    IF (
        NEW IS DISTINCT FROM OLD AND
        NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at
    ) THEN
        NEW.updated_at = CURRENT_TIMESTAMP;
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION set_updated_at_ignore_downloads() RETURNS trigger AS $$
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
$$ LANGUAGE plpgsql;
