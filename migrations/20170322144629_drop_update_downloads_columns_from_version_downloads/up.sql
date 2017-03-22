ALTER TABLE version_downloads DROP COLUMN id;
ALTER TABLE version_downloads DROP COLUMN counted;
ALTER TABLE version_downloads DROP COLUMN processed;
DROP INDEX version_downloads_unique;
ALTER TABLE version_downloads ADD PRIMARY KEY (version_id, date);
