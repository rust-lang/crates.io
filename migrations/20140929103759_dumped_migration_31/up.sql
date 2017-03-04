CREATE TABLE version_downloads (
            id              SERIAL PRIMARY KEY,
            version_id      INTEGER NOT NULL,
            downloads       INTEGER NOT NULL,
            counted         INTEGER NOT NULL,
            date            TIMESTAMP NOT NULL,
            processed       BOOLEAN NOT NULL
        );