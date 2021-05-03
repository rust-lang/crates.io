CREATE INDEX IF NOT EXISTS index_version_downloads_not_processed ON version_downloads (processed) WHERE NOT processed;
