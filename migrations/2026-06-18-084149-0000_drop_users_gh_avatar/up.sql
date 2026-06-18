-- safety-assured:start
-- The column is no longer read or written. Avatars are stored in `oauth_github.avatar`.
ALTER TABLE users DROP COLUMN gh_avatar;
-- safety-assured:end
