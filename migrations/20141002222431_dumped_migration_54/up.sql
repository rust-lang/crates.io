ALTER TABLE version_downloads ADD CONSTRAINT fk_version_downloads_version_id
                                 FOREIGN KEY (version_id) REFERENCES versions (id);