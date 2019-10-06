CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX index_crates_name_tgrm ON crates USING gin (canon_crate_name(name) gin_trgm_ops);
