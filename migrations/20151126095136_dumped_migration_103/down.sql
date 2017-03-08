ALTER TABLE version_downloads ALTER downloads DROP DEFAULT;
            ALTER TABLE version_downloads ALTER counted DROP DEFAULT;
            ALTER TABLE version_downloads ALTER date DROP DEFAULT;
            ALTER TABLE version_downloads ALTER processed DROP DEFAULT;

            ALTER TABLE keywords ALTER created_at DROP DEFAULT;
            ALTER TABLE keywords ALTER crates_cnt DROP DEFAULT;

            ALTER TABLE crates ALTER created_at DROP DEFAULT;
            ALTER TABLE crates ALTER updated_at DROP DEFAULT;
            ALTER TABLE crates ALTER downloads DROP DEFAULT;
            ALTER TABLE crates ALTER max_version DROP DEFAULT;

            ALTER TABLE crate_owners ALTER created_at DROP DEFAULT;
            ALTER TABLE crate_owners ALTER updated_at DROP DEFAULT;
            ALTER TABLE crate_owners ALTER deleted DROP DEFAULT;

            ALTER TABLE versions ALTER created_at DROP DEFAULT;
            ALTER TABLE versions ALTER updated_at DROP DEFAULT;
            ALTER TABLE versions ALTER downloads DROP DEFAULT;

            DROP TRIGGER trigger_update_keywords_crates_cnt ON crates_keywords;
            DROP FUNCTION update_keywords_crates_cnt();

            DROP TRIGGER trigger_crate_owners_set_updated_at ON crate_owners;
            DROP TRIGGER trigger_crates_set_updated_at ON crates;
            DROP TRIGGER trigger_versions_set_updated_at ON versions;

            DROP FUNCTION set_updated_at();