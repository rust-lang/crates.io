ALTER TABLE versions
    ADD CONSTRAINT versions_tar_sha256_present
    CHECK (tar_sha256 IS NOT NULL) NOT VALID;
ALTER TABLE versions ALTER COLUMN tar_sha256 DROP NOT NULL;
