ALTER TABLE crate_owners ADD CONSTRAINT fk_crate_owners_created_by
                                 FOREIGN KEY (created_by) REFERENCES users (id);