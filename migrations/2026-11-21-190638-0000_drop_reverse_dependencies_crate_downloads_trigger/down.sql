COMMENT ON TABLE reverse_dependencies IS
    'Precomputed reverse-dependency edges (target crate <- dependent crate''s default version). Derived cache table maintained by database triggers.';

CREATE FUNCTION reverse_dependencies_crate_downloads_update() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    UPDATE reverse_dependencies
    SET dependent_downloads = updated.downloads
    FROM updated
    WHERE reverse_dependencies.dependent_crate_id = updated.crate_id
      AND reverse_dependencies.dependent_downloads IS DISTINCT FROM updated.downloads;
    RETURN NULL;
END;
$$;

CREATE TRIGGER reverse_dependencies_crate_downloads_update
    AFTER UPDATE ON crate_downloads
    REFERENCING NEW TABLE AS updated
    FOR EACH STATEMENT
    EXECUTE FUNCTION reverse_dependencies_crate_downloads_update();
