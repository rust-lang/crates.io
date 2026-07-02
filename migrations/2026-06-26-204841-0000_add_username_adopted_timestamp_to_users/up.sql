ALTER TABLE users ADD COLUMN IF NOT EXISTS
    current_username_adopted_at TIMESTAMPTZ DEFAULT NULL;

COMMENT ON COLUMN users.current_username_adopted_at IS
    'Time when this user began using their current username (may be NULL if not available)';
