ALTER TABLE crate_owners ADD CONSTRAINT fk_crate_owners_crate_id
                                 FOREIGN KEY (crate_id) REFERENCES crates (id);