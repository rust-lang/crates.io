-- safety-assured:start
-- The column is no longer read or written. Encrypted GitHub tokens are stored in
-- `oauth_github.encrypted_token`.
ALTER TABLE users DROP COLUMN gh_encrypted_token;
-- safety-assured:end
