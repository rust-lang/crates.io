CREATE UNIQUE INDEX badges_crate_type ON badges (crate_id, badge_type);
ALTER TABLE badges DROP CONSTRAINT badges_pkey;
ALTER TABLE crates_categories DROP CONSTRAINT crates_categories_pkey;
ALTER TABLE crates_keywords DROP CONSTRAINT crates_keywords_pkey;
ALTER TABLE follows DROP CONSTRAINT follows_pkey;
ALTER TABLE metadata DROP CONSTRAINT metadata_pkey;
