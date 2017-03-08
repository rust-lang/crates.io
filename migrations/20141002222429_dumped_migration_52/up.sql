ALTER TABLE dependencies ADD CONSTRAINT fk_dependencies_crate_id
                                 FOREIGN KEY (crate_id) REFERENCES crates (id);