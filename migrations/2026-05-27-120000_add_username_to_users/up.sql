ALTER TABLE users ADD COLUMN IF NOT EXISTS username VARCHAR;
COMMENT ON COLUMN users.username IS 'Username associated with the user''s crates.io account, independent of linked OAuth usernames.';
