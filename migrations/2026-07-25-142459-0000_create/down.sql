CREATE INDEX IF NOT EXISTS index_users_username on users(canon_username(username))
