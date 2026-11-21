DROP TRIGGER reverse_dependencies_crate_downloads_update ON crate_downloads;
DROP FUNCTION reverse_dependencies_crate_downloads_update();

COMMENT ON TABLE reverse_dependencies IS
    'Precomputed reverse-dependency edges (target crate <- dependent crate''s default version). Derived cache table maintained by database triggers, except for `dependent_downloads`, which the `update_downloads` background job syncs.';
