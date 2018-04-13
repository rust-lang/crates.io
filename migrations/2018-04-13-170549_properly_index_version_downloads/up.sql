CREATE UNIQUE INDEX index_version_downloads_not_processed
  ON version_downloads (id)
  WHERE processed = FALSE;
DROP INDEX index_version_downloads_processed;
