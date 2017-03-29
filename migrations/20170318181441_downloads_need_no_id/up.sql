ALTER TABLE crate_downloads DROP COLUMN id;
ALTER TABLE crate_downloads ALTER COLUMN "date" TYPE date;
ALTER TABLE crate_downloads ADD PRIMARY KEY (crate_id, date);
