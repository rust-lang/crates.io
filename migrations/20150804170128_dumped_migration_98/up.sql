CREATE TABLE teams (
            id            SERIAL PRIMARY KEY,
            login         VARCHAR NOT NULL UNIQUE,
            github_id     INTEGER NOT NULL UNIQUE,
            name          VARCHAR,
            avatar        VARCHAR
        );