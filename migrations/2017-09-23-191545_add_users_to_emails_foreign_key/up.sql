ALTER TABLE emails ADD CONSTRAINT fk_emails_user_id FOREIGN KEY (user_id) REFERENCES users (id);
