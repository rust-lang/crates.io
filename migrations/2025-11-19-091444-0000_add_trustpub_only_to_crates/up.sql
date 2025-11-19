ALTER TABLE crates ADD COLUMN trustpub_only BOOLEAN NOT NULL DEFAULT FALSE;
COMMENT ON COLUMN crates.trustpub_only IS 'When true, this crate can only be published via Trusted Publishing, not with API tokens';
