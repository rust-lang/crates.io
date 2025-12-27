-- Create a table to batch pending syncs to the Git crate index.
CREATE TABLE git_index_sync_queue (
    crate_name TEXT PRIMARY KEY NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE git_index_sync_queue IS 'Queue for crates that need to be synced to the Git index';

COMMENT ON COLUMN git_index_sync_queue.crate_name IS 'The name of the crate to be synced';

COMMENT ON COLUMN git_index_sync_queue.created_at IS 'Timestamp when the sync was queued';

-- Index for efficient batch processing (oldest first).
CREATE INDEX idx_git_index_sync_queue_created_at ON git_index_sync_queue (created_at);
