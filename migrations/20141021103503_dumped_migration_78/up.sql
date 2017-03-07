CREATE TABLE keywords (
            id               SERIAL PRIMARY KEY,
            keyword          VARCHAR NOT NULL UNIQUE,
            crates_cnt       INTEGER NOT NULL,
            created_at       TIMESTAMP NOT NULL
        );