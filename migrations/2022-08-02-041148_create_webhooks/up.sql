-- Your SQL goes here
CREATE TABLE webhooks (
    id BIGSERIAL PRIMARY KEY,
    owner_id INTEGER NOT NULL,
    -- crate_id INTEGER NOT NULL,
    webhook_url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- CONSTRAINT fk_webhooks_crate_id
    --     FOREIGN KEY(crate_id)
    --         REFERENCES crates(id) ON DELETE CASCADE,
    CONSTRAINT fk_crate_owners_owner
        FOREIGN KEY(owner_id)
            REFERENCES users(id) ON DELETE CASCADE
);