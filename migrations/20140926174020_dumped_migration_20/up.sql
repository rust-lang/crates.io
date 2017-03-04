ALTER TABLE packages RENAME TO crates;
ALTER TABLE versions RENAME COLUMN package_id TO crate_id;