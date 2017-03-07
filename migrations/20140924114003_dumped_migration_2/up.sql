CREATE TABLE packages (
            id              SERIAL PRIMARY KEY,
            name            VARCHAR NOT NULL UNIQUE,
            user_id         INTEGER NOT NULL
        );