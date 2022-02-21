CREATE TABLE persistent_sessions
(
    id SERIAL
        CONSTRAINT persistent_sessions_pk
            PRIMARY KEY,
    user_id INTEGER NOT NULL
        CONSTRAINT persistent_sessions_users_id_fk
            REFERENCES users
            ON UPDATE CASCADE ON DELETE CASCADE,
    hashed_token bytea NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_used_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    revoked BOOLEAN DEFAULT FALSE NOT NULL,
    last_ip_address inet NOT NULL,
    last_user_agent VARCHAR NOT NULL
);

COMMENT ON TABLE persistent_sessions IS 'This table contains the hashed tokens for all of the cookie-based persistent sessions';

CREATE INDEX persistent_sessions_user_id_index
    ON persistent_sessions (user_id);
