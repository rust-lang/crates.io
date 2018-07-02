CREATE TABLE readme_renderings (
  version_id INTEGER NOT NULL PRIMARY KEY REFERENCES versions (id) ON DELETE CASCADE,
  rendered_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO readme_renderings (version_id, rendered_at)
  SELECT id, readme_rendered_at FROM versions WHERE readme_rendered_at IS NOT NULL;

ALTER TABLE versions DROP COLUMN readme_rendered_at;
