CREATE INDEX IF NOT EXISTS index_versions_crate_id_semver_no_prerelease_id ON versions (crate_id, semver_no_prerelease DESC NULLS LAST, id DESC) WHERE NOT yanked;
CREATE INDEX IF NOT EXISTS index_crates_id_downloads_name ON crates (id, downloads, name);
