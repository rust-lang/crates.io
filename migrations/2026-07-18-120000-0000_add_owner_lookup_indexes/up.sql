CREATE INDEX CONCURRENTLY IF NOT EXISTS index_users_username ON users (canon_username(username));
CREATE INDEX CONCURRENTLY IF NOT EXISTS index_oauth_github_login ON oauth_github (lower(login));
