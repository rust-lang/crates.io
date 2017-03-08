DROP TRIGGER trigger_crates_set_updated_at ON crates;
                DROP TRIGGER trigger_versions_set_updated_at ON versions;
                DROP FUNCTION set_updated_at_ignore_downloads();
                CREATE TRIGGER trigger_crates_set_updated_at BEFORE UPDATE
                ON crates
                FOR EACH ROW EXECUTE PROCEDURE set_updated_at();

                CREATE TRIGGER trigger_versions_set_updated_at BEFORE UPDATE
                ON versions
                FOR EACH ROW EXECUTE PROCEDURE set_updated_at();