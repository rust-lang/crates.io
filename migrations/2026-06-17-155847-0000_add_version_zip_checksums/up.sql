ALTER TABLE versions
    ADD COLUMN IF NOT EXISTS zip_sha256 BYTEA NULL
        CONSTRAINT versions_zip_sha256_len CHECK (octet_length(zip_sha256) = 32),
    ADD COLUMN IF NOT EXISTS zip_json_sha256 BYTEA NULL
        CONSTRAINT versions_zip_json_sha256_len CHECK (octet_length(zip_json_sha256) = 32);

COMMENT ON COLUMN versions.zip_sha256 IS 'SHA256 checksum of the zip source archive, or `NULL` if it has not been built yet.';
COMMENT ON COLUMN versions.zip_json_sha256 IS 'SHA256 checksum of the zip source archive manifest, or `NULL` if it has not been built yet.';
