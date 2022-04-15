CREATE TABLE persistent_sessions
(
    id BIGSERIAL
      CONSTRAINT persistent_sessions_pk
        PRIMARY KEY,
    user_id INTEGER NOT NULL,
    hashed_token bytea NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_used_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    revoked BOOLEAN DEFAULT FALSE NOT NULL,
    last_ip_address inet NOT NULL,
    last_user_agent VARCHAR NOT NULL
);

COMMENT ON TABLE persistent_sessions IS 'This table contains the hashed tokens for all of the cookie-based persistent sessions';
