CREATE INDEX CONCURRENTLY IF NOT EXISTS lower_gh_login ON users (lower(gh_login));
