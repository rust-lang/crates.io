DROP INDEX IF EXISTS index_version_downloads_by_date;
CREATE INDEX IF NOT EXISTS index_version_downloads_date ON version_downloads USING brin (date) WITH (pages_per_range = 1);
