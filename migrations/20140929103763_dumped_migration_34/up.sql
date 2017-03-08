CREATE INDEX index_version_downloads_processed ON version_downloads (processed)
                        WHERE processed = FALSE;