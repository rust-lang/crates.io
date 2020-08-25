ALTER TABLE users
    ADD COLUMN account_lock_reason VARCHAR DEFAULT NULL,
    ADD COLUMN account_lock_until TIMESTAMP DEFAULT NULL;
