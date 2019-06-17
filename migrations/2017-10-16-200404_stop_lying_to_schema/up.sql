ALTER TABLE crates DROP COLUMN max_version;
ALTER TABLE crates ALTER COLUMN textsearchable_index_col SET NOT NULL;
ALTER TABLE users DROP COLUMN api_token;
