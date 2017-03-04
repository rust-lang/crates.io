CREATE FUNCTION set_updated_at_ignore_downloads() RETURNS trigger AS $$
                BEGIN
                    IF (NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at AND
                        NEW.downloads = OLD.downloads) THEN
                        NEW.updated_at := current_timestamp;
                    END IF;
                    RETURN NEW;
                END
                $$ LANGUAGE plpgsql;

                DROP TRIGGER trigger_crates_set_updated_at ON crates;
                DROP TRIGGER trigger_versions_set_updated_at ON versions;
                CREATE TRIGGER trigger_crates_set_updated_at BEFORE UPDATE
                ON crates
                FOR EACH ROW EXECUTE PROCEDURE set_updated_at_ignore_downloads();

                CREATE TRIGGER trigger_versions_set_updated_at BEFORE UPDATE
                ON versions
                FOR EACH ROW EXECUTE PROCEDURE set_updated_at_ignore_downloads();