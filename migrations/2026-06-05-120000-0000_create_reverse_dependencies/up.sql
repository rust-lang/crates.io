-- Precomputed reverse-dependency edges: one row per `(target crate, dependent
-- crate)` where the dependent's current, non-yanked default version depends on
-- the target. Lets the serving endpoint answer a page with a bounded index range
-- scan instead of recomputing the full dependent set on every request.
--
-- The `reverse_dependencies` table is maintained entirely by the triggers below.
-- A dependent crate's edges are determined by three inputs, and a trigger reacts
-- to each: which version is its default (`default_versions`), whether that default
-- is yanked (`versions.yanked`), and that version's `dependencies` rows. Crate
-- deletions are handled by the `ON DELETE CASCADE` foreign keys, not by these triggers.


-- safety-assured:start
--
-- The `integer` FKs match the referenced `crates.id`/`versions.id` `integer`
-- primary keys, so the "widen to bigint" suggestion does not apply. The indexes
-- are built on a brand-new, empty table inside this transaction, so the `SHARE`
-- lock is instant and `CONCURRENTLY` is unnecessary (and cannot run here anyway).

CREATE TABLE IF NOT EXISTS reverse_dependencies (
    target_crate_id     integer NOT NULL REFERENCES crates (id) ON DELETE CASCADE,
    dependent_crate_id  integer NOT NULL REFERENCES crates (id) ON DELETE CASCADE,
    version_id          integer NOT NULL REFERENCES versions (id) ON DELETE CASCADE,
    -- No `REFERENCES dependencies (id)`: the `version_id` cascade already removes
    -- this row when the dependency is deleted.
    dependency_id       integer NOT NULL,
    dependent_downloads bigint  NOT NULL DEFAULT 0,
    PRIMARY KEY (target_crate_id, dependent_crate_id)
);

COMMENT ON TABLE reverse_dependencies IS
    'Precomputed reverse-dependency edges (target crate <- dependent crate''s default version). Derived cache table maintained by database triggers.';
COMMENT ON COLUMN reverse_dependencies.target_crate_id IS
    'The crate that is being depended upon.';
COMMENT ON COLUMN reverse_dependencies.dependent_crate_id IS
    'The crate whose default version depends on the target crate.';
COMMENT ON COLUMN reverse_dependencies.version_id IS
    'The dependent crate''s default version.';
COMMENT ON COLUMN reverse_dependencies.dependency_id IS
    'The `dependencies` row used to hydrate the response.';
COMMENT ON COLUMN reverse_dependencies.dependent_downloads IS
    'The dependent crate''s total downloads, used for ordering the reverse dependencies.';


CREATE INDEX IF NOT EXISTS reverse_dependencies_serve_idx
    ON reverse_dependencies (target_crate_id, dependent_downloads DESC, dependent_crate_id DESC);

CREATE INDEX IF NOT EXISTS reverse_dependencies_dependent_crate_id_idx
    ON reverse_dependencies (dependent_crate_id);

CREATE INDEX IF NOT EXISTS reverse_dependencies_version_id_idx
    ON reverse_dependencies (version_id);

-- safety-assured:end


CREATE FUNCTION compute_reverse_dependencies(dependent_crate_ids integer[])
RETURNS TABLE (
    target_crate_id integer,
    dependent_crate_id integer,
    version_id integer,
    dependency_id integer,
    dependent_downloads bigint
)
LANGUAGE sql STABLE AS $$
    SELECT dependencies.crate_id        AS target_crate_id,
           default_versions.crate_id    AS dependent_crate_id,
           default_versions.version_id  AS version_id,
           MIN(dependencies.id)         AS dependency_id,
           COALESCE((SELECT downloads FROM crate_downloads
                     WHERE crate_downloads.crate_id = default_versions.crate_id), 0) AS dependent_downloads
    FROM default_versions
    JOIN versions ON versions.id = default_versions.version_id AND NOT versions.yanked
    JOIN dependencies ON dependencies.version_id = default_versions.version_id
    WHERE default_versions.crate_id = ANY(dependent_crate_ids)
    GROUP BY dependencies.crate_id, default_versions.crate_id;
$$;

COMMENT ON FUNCTION compute_reverse_dependencies(integer[]) IS
    'Returns the rows that should exist in `reverse_dependencies` for the given dependent crate ids, one per target crate.';


CREATE FUNCTION rebuild_reverse_dependencies(dependent_crate_ids integer[]) RETURNS void
LANGUAGE sql VOLATILE AS $$
    -- Clear the crates' current rows, then write the freshly computed set. These
    -- are two statements rather than one data-modifying CTE: a CTE would run both
    -- against the same snapshot, so the INSERT would still see the about-to-be
    -- deleted rows and trip the primary key. A crate that computes to no rows is
    -- left empty by the DELETE.

    DELETE FROM reverse_dependencies WHERE dependent_crate_id = ANY(dependent_crate_ids);

    INSERT INTO reverse_dependencies
        (target_crate_id, dependent_crate_id, version_id, dependency_id, dependent_downloads)
    SELECT target_crate_id, dependent_crate_id, version_id, dependency_id, dependent_downloads
    FROM compute_reverse_dependencies(dependent_crate_ids);
$$;

COMMENT ON FUNCTION rebuild_reverse_dependencies(integer[]) IS
    'Rebuilds the `reverse_dependencies` rows for the given dependent crate ids.';

--
-- AFTER INSERT ON default_versions
--

CREATE FUNCTION reverse_dependencies_default_versions_insert() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM rebuild_reverse_dependencies(ARRAY(SELECT crate_id FROM inserted));
    RETURN NULL;
END;
$$;

CREATE TRIGGER reverse_dependencies_default_versions_insert
    AFTER INSERT ON default_versions
    REFERENCING NEW TABLE AS inserted
    FOR EACH STATEMENT
    EXECUTE FUNCTION reverse_dependencies_default_versions_insert();

--
-- AFTER UPDATE ON default_versions
--

CREATE FUNCTION reverse_dependencies_default_versions_update() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM rebuild_reverse_dependencies(ARRAY(
        SELECT new_default_versions.crate_id
        FROM new_default_versions
        JOIN old_default_versions ON old_default_versions.crate_id = new_default_versions.crate_id
        WHERE new_default_versions.version_id IS DISTINCT FROM old_default_versions.version_id
    ));
    RETURN NULL;
END;
$$;

CREATE TRIGGER reverse_dependencies_default_versions_update
    AFTER UPDATE ON default_versions
    REFERENCING OLD TABLE AS old_default_versions NEW TABLE AS new_default_versions
    FOR EACH STATEMENT
    EXECUTE FUNCTION reverse_dependencies_default_versions_update();

--
-- AFTER UPDATE OF yanked ON versions
--

CREATE FUNCTION reverse_dependencies_versions_yanked() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM rebuild_reverse_dependencies(ARRAY(
        SELECT crate_id FROM default_versions WHERE version_id = NEW.id
    ));
    RETURN NULL;
END;
$$;

CREATE TRIGGER reverse_dependencies_versions_yanked
    AFTER UPDATE OF yanked ON versions
    FOR EACH ROW
    WHEN (OLD.yanked IS DISTINCT FROM NEW.yanked)
    EXECUTE FUNCTION reverse_dependencies_versions_yanked();

--
-- AFTER INSERT ON dependencies
--

CREATE FUNCTION reverse_dependencies_dependencies_insert() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM rebuild_reverse_dependencies(ARRAY(
        SELECT DISTINCT default_versions.crate_id
        FROM default_versions
        JOIN inserted ON inserted.version_id = default_versions.version_id
    ));
    RETURN NULL;
END;
$$;

CREATE TRIGGER reverse_dependencies_dependencies_insert
    AFTER INSERT ON dependencies
    REFERENCING NEW TABLE AS inserted
    FOR EACH STATEMENT
    EXECUTE FUNCTION reverse_dependencies_dependencies_insert();

--
-- AFTER UPDATE OF downloads ON crate_downloads
--

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
