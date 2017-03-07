ALTER TABLE version_authors ADD CONSTRAINT fk_version_authors_version_id
                                 FOREIGN KEY (version_id) REFERENCES versions (id);