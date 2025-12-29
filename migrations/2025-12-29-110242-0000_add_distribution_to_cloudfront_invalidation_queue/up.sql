ALTER TABLE cloudfront_invalidation_queue
    ADD COLUMN distribution TEXT NOT NULL DEFAULT 'index';

COMMENT ON COLUMN cloudfront_invalidation_queue.distribution IS 'CloudFront distribution to invalidate: "index" for index.crates.io, "static" for static.crates.io';
