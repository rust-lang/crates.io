CREATE UNIQUE INDEX reserved_crate_names_canon_crate_name_idx ON reserved_crate_names (canon_crate_name(name));
CREATE INDEX index_users_gh_id ON users (gh_id);
CREATE INDEX index_version_downloads_version_id ON version_downloads (version_id);
CREATE INDEX index_version_downloads_date ON version_downloads (date);
