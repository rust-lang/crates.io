ALTER TABLE users
ADD COLUMN IF NOT EXISTS session_epoch INTEGER NOT NULL DEFAULT 0;

COMMENT ON COLUMN users.session_epoch IS 'The minimum epoch required for a session cookie to be valid.';
