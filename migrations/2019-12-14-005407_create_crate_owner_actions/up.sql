CREATE TABLE crate_owner_actions (
    id SERIAL PRIMARY KEY,
    crate_id INTEGER NOT NULL REFERENCES crates(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id),
    api_token_id INTEGER REFERENCES api_tokens(id),
    action INTEGER NOT NULL,
    time TIMESTAMP NOT NULL DEFAULT now()
);
