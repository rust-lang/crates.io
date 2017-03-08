CREATE TABLE crate_downloads (
            id              SERIAL PRIMARY KEY,
            crate_id        INTEGER NOT NULL,
            downloads       INTEGER NOT NULL,
            date            TIMESTAMP NOT NULL
        );