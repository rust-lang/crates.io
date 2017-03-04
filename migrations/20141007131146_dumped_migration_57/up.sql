CREATE TABLE version_authors (
            id               SERIAL PRIMARY KEY,
            version_id       INTEGER NOT NULL,
            user_id          INTEGER,
            name             VARCHAR NOT NULL
        );