-- remove the old trigger
DROP TRIGGER trigger_versions_set_updated_at ON versions;

-- add the new trigger only for the `yanked` column
CREATE TRIGGER trigger_versions_set_updated_at
    BEFORE UPDATE OF yanked
    ON versions
    FOR EACH ROW
EXECUTE PROCEDURE set_updated_at();
