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
        SELECT versions.*,
        row_number() OVER (
            PARTITION BY crate_id
            ORDER BY to_semver_no_prerelease(num) DESC NULLS LAST
        ) rn
        FROM versions
        WHERE NOT yanked
        -- This is completely redundant, but it's faster to filter the versions
        -- early even if this subselect is done via an index scan.
        AND crate_id = ANY(
            SELECT versions.crate_id
            FROM versions
            INNER JOIN dependencies
            ON dependencies.version_id = versions.id
            WHERE dependencies.crate_id = $1
        )
    ) versions
      ON versions.id = dependencies.version_id
    INNER JOIN crates
      ON crates.id = versions.crate_id
    WHERE dependencies.crate_id = $1
      AND rn = 1
    ORDER BY crate_downloads DESC
) t
OFFSET $2
LIMIT $3
