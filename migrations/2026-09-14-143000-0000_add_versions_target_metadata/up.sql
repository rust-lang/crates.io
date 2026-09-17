ALTER TABLE versions ADD COLUMN IF NOT EXISTS target_metadata JSONB;

COMMENT ON COLUMN versions.target_metadata IS 'Build script, library, and binary target metadata derived from the published package';
