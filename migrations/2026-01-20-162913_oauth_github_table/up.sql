-- safety-assured:start
CREATE TABLE IF NOT EXISTS oauth_github (
  -- Corresponds to users.gh_id. Even though users.gh_id is INTEGER and not BIGINT, the GitHub
  -- API documentation says its IDs are int64, so let's future-proof the table while we migrate.
  account_id BIGINT NOT NULL PRIMARY KEY,
  -- Safe (and required) to use INTEGER rather than BIGINT because this is a foreign key to
  -- users.id, which is INTEGER
  user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  -- Corresponds to users.gh_encrypted_token
  encrypted_token bytea NOT NULL,
  -- Corresponds to users.gh_login
  login VARCHAR NOT NULL,
  -- Corresponds to users.gh_avatar
  avatar VARCHAR
);
-- safety-assured:end

comment on table oauth_github is 'GitHub-specific account information associated with a crates.io account';
comment on column oauth_github.account_id is 'GitHub ID returned from the oAuth response';
comment on column oauth_github.user_id is 'Crates.io user ID foreign key';
comment on column oauth_github.encrypted_token is 'Encrypted GitHub access token';
comment on column oauth_github.login is 'GitHub username';
comment on column oauth_github.avatar is 'GitHub avatar URL';
