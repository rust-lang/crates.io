ALTER TABLE crates DROP CONSTRAINT packages_name_key;
CREATE UNIQUE INDEX index_crates_name ON crates (lower(name));