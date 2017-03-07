ALTER TABLE version_authors ADD CONSTRAINT fk_version_authors_user_id
                                 FOREIGN KEY (user_id) REFERENCES users (id);