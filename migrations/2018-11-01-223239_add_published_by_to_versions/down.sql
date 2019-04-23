ALTER TABLE versions DROP CONSTRAINT fk_versions_published_by;

ALTER TABLE versions DROP COLUMN published_by;
