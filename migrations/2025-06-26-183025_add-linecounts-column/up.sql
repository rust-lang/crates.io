-- Add line count statistics column to versions table
ALTER TABLE versions
ADD COLUMN linecounts JSONB DEFAULT NULL;

-- Add comment explaining the column
COMMENT ON COLUMN versions.linecounts IS 'Source Lines of Code statistics for this version, stored as JSON with language breakdown and totals.';
