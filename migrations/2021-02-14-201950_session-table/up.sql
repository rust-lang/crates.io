CREATE TABLE sessions
(
    id SERIAL
        CONSTRAINT sessions_pk
            PRIMARY KEY,
    user_id INTEGER NOT NULL
        CONSTRAINT sessions_users_id_fk
            REFERENCES users
            ON UPDATE CASCADE ON DELETE CASCADE,
    hashed_token bytea NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_used_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    revoked BOOLEAN DEFAULT FALSE NOT NULL,
    last_ip_address inet NOT NULL,
    last_user_agent VARCHAR NOT NULL
);

COMMENT ON TABLE sessions IS 'This table contains the tokens for all of the cookie-based sessions';

CREATE INDEX sessions_user_id_index
    ON sessions (user_id);

CREATE UNIQUE INDEX sessions_token_uindex
	ON sessions (hashed_token);
