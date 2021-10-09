-- add the new column.

ALTER TABLE versions
    ADD COLUMN semver_no_prerelease NUMERIC[3];
