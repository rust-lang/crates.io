DROP INDEX index_crates_name;
CREATE UNIQUE INDEX index_crates_name ON crates (lower(name));