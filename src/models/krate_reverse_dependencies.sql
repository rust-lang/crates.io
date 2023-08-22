-- Apply pagination to the whole thing
SELECT *, COUNT(*) OVER () as total FROM (
    -- Multple dependencies can exist, make it distinct
    SELECT DISTINCT ON (crate_downloads, crate_name)
    dependencies.*,
    crates.downloads AS crate_downloads,
    crates.name AS crate_name
    FROM dependencies
    -- We only want the crates whose *max* version is dependent, so we join on a
    -- subselect that includes the versions with their ordinal position
    INNER JOIN (
        SELECT DISTINCT ON (crate_id)
           *
        FROM versions
        WHERE NOT yanked
        ORDER BY
            crate_id,
            semver_no_prerelease DESC NULLS LAST,
            id DESC
    ) versions
      ON versions.id = dependencies.version_id
    INNER JOIN crates
      ON crates.id = versions.crate_id
    WHERE dependencies.crate_id = $1
    -- this ORDER BY is redundant with the outer one but benefits
    -- the `DISTINCT ON`
    ORDER BY
        crate_downloads DESC,
        crate_name ASC,
        dependencies.id ASC
) t
ORDER BY
    crate_downloads DESC,
    crate_name ASC
OFFSET $2
LIMIT $3
