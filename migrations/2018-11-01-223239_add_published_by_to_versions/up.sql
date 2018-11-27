ALTER TABLE versions
ADD COLUMN published_by integer;

ALTER TABLE versions
ADD CONSTRAINT "fk_versions_published_by"
FOREIGN KEY (published_by)
REFERENCES users(id);
