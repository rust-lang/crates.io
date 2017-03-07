CREATE TABLE dependencies (
            id               SERIAL PRIMARY KEY,
            version_id       INTEGER NOT NULL,
            crate_id         INTEGER NOT NULL,
            req              VARCHAR NOT NULL,
            optional         BOOLEAN NOT NULL,
            default_features BOOLEAN NOT NULL,
            features         VARCHAR NOT NULL
        );