ALTER TABLE versions ADD COLUMN IF NOT EXISTS tar_sha256 bytea;

COMMENT ON COLUMN versions.tar_sha256 IS 'SHA256 checksum of the crate tarball, stored as 32 raw bytes.';

ALTER TABLE versions
    ADD CONSTRAINT versions_tar_sha256_len
    CHECK (octet_length(tar_sha256) = 32) NOT VALID;
