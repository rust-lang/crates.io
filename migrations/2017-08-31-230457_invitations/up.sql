CREATE TABLE crate_owner_invitations (
    invited_user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    invited_by_user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    crate_id INTEGER NOT NULL REFERENCES crates (id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    PRIMARY KEY (invited_user_id, crate_id)
);
