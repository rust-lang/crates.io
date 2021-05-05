DROP INDEX index_version_downloads_date;
CREATE INDEX IF NOT EXISTS index_version_downloads_by_date ON version_downloads USING brin (date);
