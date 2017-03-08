ALTER TABLE follows ADD CONSTRAINT fk_follows_crate_id
                                 FOREIGN KEY (crate_id) REFERENCES crates (id);