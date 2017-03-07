ALTER TABLE crate_downloads ADD CONSTRAINT fk_crate_downloads_crate_id
                                 FOREIGN KEY (crate_id) REFERENCES crates (id);