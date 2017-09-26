ALTER TABLE versions ADD COLUMN readme_rendered_at TIMESTAMP;
UPDATE versions SET readme_rendered_at = readme_rendering.rendered_at
  FROM readme_rendering WHERE readme_rendering.version_id = versions.id;
DROP TABLE readme_rendering;
