CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_trustpub_configs_github_repo
ON trustpub_configs_github (LOWER(repository_owner), LOWER(repository_name));
