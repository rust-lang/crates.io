CREATE TABLE IF NOT EXISTS metadata_github_refresh (
    highest_processed_user_id INTEGER NOT NULL DEFAULT 0 PRIMARY KEY,
    batch_size BIGINT NOT NULL DEFAULT 100
);

COMMENT ON TABLE metadata_github_refresh
IS 'Track where we are in refreshing user GitHub info from the GitHub API in batches in jobs';
COMMENT ON COLUMN metadata_github_refresh.highest_processed_user_id
IS 'The highest crates.io user ID that was processed in a previously completed batch. The next run will request a batch of users with IDs greater than this.';
COMMENT ON COLUMN metadata_github_refresh.batch_size
IS 'The number of users to request in the next batch.';

INSERT INTO metadata_github_refresh (highest_processed_user_id, batch_size)
VALUES (0, 100)
ON CONFLICT DO NOTHING;
