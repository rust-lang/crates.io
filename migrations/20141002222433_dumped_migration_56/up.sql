ALTER TABLE follows ADD CONSTRAINT fk_follows_user_id
                                 FOREIGN KEY (user_id) REFERENCES users (id);