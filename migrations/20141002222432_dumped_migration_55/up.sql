ALTER TABLE versions ADD CONSTRAINT fk_versions_crate_id
                                 FOREIGN KEY (crate_id) REFERENCES crates (id);