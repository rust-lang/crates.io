ALTER TABLE default_versions
    DROP COLUMN num_versions;

DROP FUNCTION IF EXISTS update_num_versions_from_versions CASCADE;
