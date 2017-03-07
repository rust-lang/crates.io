ALTER TABLE crates_keywords ADD CONSTRAINT fk_crates_keywords_crate_id
                                 FOREIGN KEY (crate_id) REFERENCES crates (id);