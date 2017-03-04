ALTER TABLE crates RENAME TO packages;
ALTER TABLE versions RENAME COLUMN crate_id TO package_id;