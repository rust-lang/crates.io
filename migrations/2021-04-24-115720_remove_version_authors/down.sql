CREATE TABLE version_authors(
    id SERIAL  NOT NULL CONSTRAINT version_authors_pkey PRIMARY KEY,
    version_id INTEGER NOT NULL CONSTRAINT fk_version_authors_version_id REFERENCES versions ON DELETE CASCADE,
    name VARCHAR NOT NULL
);

CREATE INDEX index_version_authors_version_id ON version_authors (version_id);
