CREATE TABLE versions_published_by (
    version_id INTEGER NOT NULL PRIMARY KEY REFERENCES versions(id) ON DELETE CASCADE,
    email VARCHAR NOT NULL
);