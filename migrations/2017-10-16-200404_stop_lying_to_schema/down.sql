ALTER TABLE crates ADD COLUMN max_version VARCHAR;
ALTER TABLE crates ALTER COLUMN textsearchable_index_col DROP NOT NULL;
ALTER TABLE users ADD COLUMN api_token VARCHAR NOT NULL DEFAULT random_string(32);
