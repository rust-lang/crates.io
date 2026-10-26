CREATE UNIQUE INDEX CONCURRENTLY users_gh_id ON users (gh_id) WHERE gh_id > 0;
