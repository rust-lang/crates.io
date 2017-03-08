ALTER TABLE crates_keywords ADD CONSTRAINT fk_crates_keywords_keyword_id
                                 FOREIGN KEY (keyword_id) REFERENCES keywords (id);