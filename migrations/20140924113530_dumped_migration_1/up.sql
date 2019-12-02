CREATE TABLE users (
            id              SERIAL PRIMARY KEY,
            email           VARCHAR NOT NULL UNIQUE,
            gh_access_token VARCHAR NOT NULL,
            api_token       VARCHAR NOT NULL
        );

