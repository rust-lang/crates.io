-- These indexes were created on production by hand and never existed in migrations; give them
-- better names now that we know we want to keep them
ALTER INDEX IF EXISTS sgrif_testing RENAME TO index_recent_crate_downloads_by_downloads;
ALTER INDEX IF EXISTS sgrif_testing2 RENAME TO index_version_downloads_by_date;

-- Create the above indexes for databases other than production
CREATE INDEX IF NOT EXISTS index_recent_crate_downloads_by_downloads
  ON recent_crate_downloads USING btree (downloads);
CREATE INDEX IF NOT EXISTS index_version_downloads_by_date ON version_downloads USING brin (date);
