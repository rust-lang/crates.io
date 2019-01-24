CREATE TABLE version_owner_actions (
    id SERIAL PRIMARY KEY,
    version_id INTEGER REFERENCES versions(id) ON DELETE CASCADE,
    owner_id INTEGER REFERENCES users(id),
    owner_token_id INTEGER REFERENCES api_tokens(id),
    action INTEGER NOT NULL,
    time TIMESTAMP NOT NULL DEFAULT now()
);
