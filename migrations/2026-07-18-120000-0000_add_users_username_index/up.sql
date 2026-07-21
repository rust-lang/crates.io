CREATE INDEX CONCURRENTLY IF NOT EXISTS index_users_username ON users (canon_username(username));
