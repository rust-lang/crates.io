ALTER TABLE "dependencies"
    DROP CONSTRAINT "fk_dependencies_version_id",
    ADD CONSTRAINT "fk_dependencies_version_id" FOREIGN KEY (version_id) REFERENCES versions(id);
ALTER TABLE "version_authors"
    DROP CONSTRAINT "fk_version_authors_version_id",
    ADD CONSTRAINT "fk_version_authors_version_id" FOREIGN KEY (version_id) REFERENCES versions(id);
ALTER TABLE "version_downloads"
    DROP CONSTRAINT "fk_version_downloads_version_id",
    ADD CONSTRAINT "fk_version_downloads_version_id" FOREIGN KEY (version_id) REFERENCES versions(id);
