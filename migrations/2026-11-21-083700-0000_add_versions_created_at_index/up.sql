CREATE INDEX CONCURRENTLY IF NOT EXISTS versions_created_at_idx
    ON versions (created_at);
