CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS index_users_username_unique ON users(canon_username(username))
