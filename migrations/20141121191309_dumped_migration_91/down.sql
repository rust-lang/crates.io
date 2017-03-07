DROP INDEX index_crates_name;
ALTER TABLE crates ADD CONSTRAINT packages_name_key UNIQUE (name);