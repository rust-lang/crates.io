CREATE TABLE readme_rendering (
    version_id INTEGER NOT NULL PRIMARY KEY,
    rendered_at TIMESTAMP WITHOUT TIME ZONE
);

ALTER TABLE readme_rendering
ADD CONSTRAINT "fk_readme_rendering_version_id"
FOREIGN KEY (version_id)
REFERENCES versions(id)
ON DELETE CASCADE;
