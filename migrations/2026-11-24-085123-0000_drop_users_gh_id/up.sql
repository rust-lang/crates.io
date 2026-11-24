-- safety-assured:start
-- The column is no longer read or written. GitHub IDs are stored in
-- `oauth_github.account_id`.
ALTER TABLE users DROP COLUMN gh_id;
-- safety-assured:end
