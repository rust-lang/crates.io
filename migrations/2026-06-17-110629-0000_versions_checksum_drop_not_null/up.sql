-- safety-assured:start
-- The previous release stopped selecting `checksum` into the `Version` read
-- model and reads the digest from `tar_sha256` instead, so no running code
-- depends on `checksum` being non-null. Making it nullable lets this release
-- stop writing it before the column is dropped.
ALTER TABLE versions ALTER COLUMN checksum DROP NOT NULL;
-- safety-assured:end

