ALTER TABLE version_downloads ALTER COLUMN "date" TYPE date;
CREATE UNIQUE INDEX version_downloads_unique ON version_downloads (version_id, date);
