ALTER TABLE crates_categories ADD CONSTRAINT fk_crates_categories_crate_id
                                 FOREIGN KEY (crate_id) REFERENCES crates (id);