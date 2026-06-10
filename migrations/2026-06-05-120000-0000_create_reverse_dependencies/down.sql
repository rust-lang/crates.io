DROP TRIGGER reverse_dependencies_crate_downloads_update ON crate_downloads;
DROP FUNCTION reverse_dependencies_crate_downloads_update();

DROP TRIGGER reverse_dependencies_dependencies_insert ON dependencies;
DROP TRIGGER reverse_dependencies_versions_yanked ON versions;
DROP TRIGGER reverse_dependencies_default_versions_update ON default_versions;
DROP TRIGGER reverse_dependencies_default_versions_insert ON default_versions;

DROP FUNCTION reverse_dependencies_dependencies_insert();
DROP FUNCTION reverse_dependencies_versions_yanked();
DROP FUNCTION reverse_dependencies_default_versions_update();
DROP FUNCTION reverse_dependencies_default_versions_insert();
DROP FUNCTION rebuild_reverse_dependencies(integer[]);
DROP FUNCTION compute_reverse_dependencies(integer[]);

DROP TABLE reverse_dependencies;
