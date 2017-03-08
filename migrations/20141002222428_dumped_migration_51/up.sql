ALTER TABLE dependencies ADD CONSTRAINT fk_dependencies_version_id
                                 FOREIGN KEY (version_id) REFERENCES versions (id);