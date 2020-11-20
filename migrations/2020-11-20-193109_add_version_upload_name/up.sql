ALTER TABLE versions
ADD COLUMN upload_name VARCHAR NOT NULL
;

UPDATE versions
SET upload_name = crates.name
FROM crates
WHERE versions.crate_id = crates.id
;
