ALTER TABLE version_downloads ALTER downloads SET DEFAULT 1;
            ALTER TABLE version_downloads ALTER counted SET DEFAULT 0;
            ALTER TABLE version_downloads ALTER date SET DEFAULT current_date;
            ALTER TABLE version_downloads ALTER processed SET DEFAULT 'f';

            ALTER TABLE keywords ALTER created_at SET DEFAULT current_timestamp;
            ALTER TABLE keywords ALTER crates_cnt SET DEFAULT 0;

            ALTER TABLE crates ALTER created_at SET DEFAULT current_timestamp;
            ALTER TABLE crates ALTER updated_at SET DEFAULT current_timestamp;
            ALTER TABLE crates ALTER downloads SET DEFAULT 0;
            ALTER TABLE crates ALTER max_version SET DEFAULT '0.0.0';

            ALTER TABLE crate_owners ALTER created_at SET DEFAULT current_timestamp;
            ALTER TABLE crate_owners ALTER updated_at SET DEFAULT current_timestamp;
            ALTER TABLE crate_owners ALTER deleted SET DEFAULT 'f';

            ALTER TABLE versions ALTER created_at SET DEFAULT current_timestamp;
            ALTER TABLE versions ALTER updated_at SET DEFAULT current_timestamp;
            ALTER TABLE versions ALTER downloads SET DEFAULT 0;

            CREATE FUNCTION set_updated_at() RETURNS trigger AS $$
            BEGIN
                IF (NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at) THEN
                    NEW.updated_at := current_timestamp;
                END IF;
                RETURN NEW;
            END
            $$ LANGUAGE plpgsql;

            CREATE FUNCTION update_keywords_crates_cnt() RETURNS trigger AS $$
            BEGIN
                IF (TG_OP = 'INSERT') THEN
                    UPDATE keywords SET crates_cnt = crates_cnt + 1 WHERE id = NEW.keyword_id;
                    return NEW;
                ELSIF (TG_OP = 'DELETE') THEN
                    UPDATE keywords SET crates_cnt = crates_cnt - 1 WHERE id = OLD.keyword_id;
                    return OLD;
                END IF;
            END
            $$ LANGUAGE plpgsql;

            CREATE TRIGGER trigger_update_keywords_crates_cnt BEFORE INSERT OR DELETE
            ON crates_keywords
            FOR EACH ROW EXECUTE PROCEDURE update_keywords_crates_cnt();

            CREATE TRIGGER trigger_crate_owners_set_updated_at BEFORE UPDATE
            ON crate_owners
            FOR EACH ROW EXECUTE PROCEDURE set_updated_at();

            CREATE TRIGGER trigger_crates_set_updated_at BEFORE UPDATE
            ON crates
            FOR EACH ROW EXECUTE PROCEDURE set_updated_at();

            CREATE TRIGGER trigger_versions_set_updated_at BEFORE UPDATE
            ON versions
            FOR EACH ROW EXECUTE PROCEDURE set_updated_at();