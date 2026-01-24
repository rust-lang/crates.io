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

comment on column oauth_github.account_id is 'Corresponds to users.gh_id';
comment on column oauth_github.encrypted_token is 'Corresponds to users.gh_encrypted_token';
comment on column oauth_github.login is 'Corresponds to users.gh_login';
comment on column oauth_github.avatar is 'Corresponds to users.gh_avatar';
