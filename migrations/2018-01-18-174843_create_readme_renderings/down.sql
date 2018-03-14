ALTER TABLE versions ADD COLUMN readme_rendered_at TIMESTAMP;

UPDATE versions SET readme_rendered_at = readme_renderings.rendered_at
  FROM readme_renderings
  WHERE readme_renderings.version_id = versions.id;

DROP TABLE readme_renderings;
