-- Add trustpub_data column to trustpub_tokens table
ALTER TABLE trustpub_tokens ADD COLUMN trustpub_data JSONB;
COMMENT ON COLUMN trustpub_tokens.trustpub_data IS 'JSONB data containing JWT claims from the trusted publisher (e.g., GitHub Actions context like repository, run_id, sha)';

-- Add trustpub_data column to versions table
ALTER TABLE versions ADD COLUMN trustpub_data JSONB;
COMMENT ON COLUMN versions.trustpub_data IS 'JSONB data containing JWT claims from the trusted publisher (e.g., GitHub Actions context like repository, run_id, sha)';
