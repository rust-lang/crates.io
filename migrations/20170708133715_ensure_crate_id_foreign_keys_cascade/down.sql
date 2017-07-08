ALTER TABLE "crate_downloads"
    DROP CONSTRAINT "fk_crate_downloads_crate_id",
    ADD CONSTRAINT "fk_crate_downloads_crate_id" FOREIGN KEY (crate_id) REFERENCES crates(id);
ALTER TABLE "crate_owners"
    DROP CONSTRAINT "fk_crate_owners_crate_id",
    ADD CONSTRAINT "fk_crate_owners_crate_id" FOREIGN KEY (crate_id) REFERENCES crates(id);
ALTER TABLE "crates_categories"
    DROP CONSTRAINT "fk_crates_categories_crate_id",
    ADD CONSTRAINT "fk_crates_categories_crate_id" FOREIGN KEY (crate_id) REFERENCES crates(id);
ALTER TABLE "crates_keywords"
    DROP CONSTRAINT "fk_crates_keywords_crate_id",
    ADD CONSTRAINT "fk_crates_keywords_crate_id" FOREIGN KEY (crate_id) REFERENCES crates(id);
ALTER TABLE "dependencies"
    DROP CONSTRAINT "fk_dependencies_crate_id",
    ADD CONSTRAINT "fk_dependencies_crate_id" FOREIGN KEY (crate_id) REFERENCES crates(id);
ALTER TABLE "follows"
    DROP CONSTRAINT "fk_follows_crate_id",
    ADD CONSTRAINT "fk_follows_crate_id" FOREIGN KEY (crate_id) REFERENCES crates(id);
ALTER TABLE "versions"
    DROP CONSTRAINT "fk_versions_crate_id",
    ADD CONSTRAINT "fk_versions_crate_id" FOREIGN KEY (crate_id) REFERENCES crates(id);
