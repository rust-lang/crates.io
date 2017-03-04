ALTER TABLE crate_owners ADD CONSTRAINT fk_crate_owners_user_id
                                 FOREIGN KEY (user_id) REFERENCES users (id);