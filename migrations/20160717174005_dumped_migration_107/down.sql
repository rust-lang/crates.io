ALTER TABLE crates ADD CONSTRAINT fk_crates_user_id
                           FOREIGN KEY (user_id) REFERENCES users (id);