DROP INDEX index_crates_name;
CREATE UNIQUE INDEX index_crates_name ON crates (canon_crate_name(name));