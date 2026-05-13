-- safety-assured:start
-- Adding a column with a constant value default is safe because we're using Postgres > 11.
ALTER TABLE IF EXISTS oauth_github
-- Set existing rows' last sync value to 1970.
ADD COLUMN last_sync timestamptz NOT NULL DEFAULT to_timestamp(0);
-- safety-assured:end

comment on column oauth_github.last_sync is 'The last time we verified with GitHub what the GitHub username for this user was, and whether the account was valid';
