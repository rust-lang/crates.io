-- Create table for queuing CloudFront invalidation paths
CREATE TABLE cloudfront_invalidation_queue (
    id BIGSERIAL PRIMARY KEY,
    path TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE cloudfront_invalidation_queue IS 'Queue for batching CloudFront CDN invalidation requests';
COMMENT ON COLUMN cloudfront_invalidation_queue.id IS 'Unique identifier for each queued invalidation path';
COMMENT ON COLUMN cloudfront_invalidation_queue.path IS 'CloudFront path to invalidate (e.g. /crates/serde/serde-1.0.0.crate)';
COMMENT ON COLUMN cloudfront_invalidation_queue.created_at IS 'Timestamp when the path was queued for invalidation';

-- Index for efficient batch processing (oldest first)
CREATE INDEX idx_cloudfront_invalidation_queue_created_at
    ON cloudfront_invalidation_queue (created_at);
