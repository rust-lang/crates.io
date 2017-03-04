DROP INDEX badges_crate_type;
ALTER TABLE badges ADD PRIMARY KEY (crate_id, badge_type);
ALTER TABLE crates_categories ADD PRIMARY KEY (crate_id, category_id);
ALTER TABLE crates_keywords ADD PRIMARY KEY (crate_id, keyword_id);
ALTER TABLE follows ADD PRIMARY KEY (user_id, crate_id);
ALTER TABLE metadata ADD PRIMARY KEY (total_downloads);
