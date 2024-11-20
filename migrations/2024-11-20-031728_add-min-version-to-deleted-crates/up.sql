ALTER TABLE deleted_crates
ADD COLUMN min_version VARCHAR NULL;

COMMENT ON COLUMN deleted_crates.min_version IS 'The first version that can be used by a new crate with the same name';
