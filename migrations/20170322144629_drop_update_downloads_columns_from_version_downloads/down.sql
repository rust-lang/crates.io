ALTER TABLE version_downloads DROP CONSTRAINT version_downloads_pkey;
ALTER TABLE version_downloads ADD COLUMN id SERIAL PRIMARY KEY;
ALTER TABLE version_downloads ADD COLUMN counted INTEGER NOT NULL DEFAULT 0;
ALTER TABLE version_downloads ADD COLUMN processed BOOLEAN NOT NULL DEFAULT 'f';
UPDATE version_downloads SET counted = downloads, processed = 't';
CREATE UNIQUE INDEX version_downloads_unique ON version_downloads (version_id, date);
