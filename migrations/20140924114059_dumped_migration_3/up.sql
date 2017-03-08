CREATE TABLE versions (
            id              SERIAL PRIMARY KEY,
            package_id      INTEGER NOT NULL,
            num             VARCHAR NOT NULL
        );