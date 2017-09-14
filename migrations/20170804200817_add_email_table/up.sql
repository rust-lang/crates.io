-- Your SQL goes here
CREATE table emails (
    id          SERIAL PRIMARY KEY,
    user_id     INTEGER NOT NULL UNIQUE,
    email       VARCHAR NOT NULL,
    verified    BOOLEAN DEFAULT false NOT NULL
);

CREATE table tokens (
    id          SERIAL PRIMARY KEY,
    email_id    INTEGER NOT NULL UNIQUE REFERENCES emails,
    token       VARCHAR NOT NULL,
    created_at  TIMESTAMP NOT NULL DEFAULT now()
);

INSERT INTO emails (user_id, email)
    SELECT id, email FROM users WHERE email IS NOT NULL;
