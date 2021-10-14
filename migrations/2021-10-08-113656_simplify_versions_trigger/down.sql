-- remove the new trigger
DROP TRIGGER trigger_versions_set_updated_at ON versions;

-- add the old trigger again
CREATE TRIGGER trigger_versions_set_updated_at
    BEFORE UPDATE
    ON versions
    FOR EACH ROW
EXECUTE PROCEDURE set_updated_at_ignore_downloads();
