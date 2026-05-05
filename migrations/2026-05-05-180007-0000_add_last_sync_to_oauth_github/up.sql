ALTER TABLE IF EXISTS oauth_github
-- Set existing rows' last sync value to 1970
ADD COLUMN last_sync timestamptz NOT NULL DEFAULT to_timestamp(0),
-- Set new rows' last sync value to the current time going forward, because creating a new
-- crates.io account fetches the user info from GitHub
ALTER COLUMN last_sync SET DEFAULT now();

comment on column oauth_github.last_sync is 'The last time we verified with GitHub what the GitHub username for this user was, and whether the account was valid';
