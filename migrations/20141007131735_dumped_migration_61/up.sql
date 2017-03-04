CREATE TABLE crate_owners (
            id               SERIAL PRIMARY KEY,
            crate_id         INTEGER NOT NULL,
            user_id          INTEGER NOT NULL,
            created_at       TIMESTAMP NOT NULL,
            created_by       INTEGER
        );