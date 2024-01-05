SELECT
    dependencies.*, crate_downloads, crate_name, total
FROM (
    -- Apply pagination to the crates
    SELECT *, COUNT(*) OVER () as total FROM (
        SELECT
            crates.downloads AS crate_downloads,
            crates.name AS crate_name,
            versions.id AS version_id
        FROM
        -- We only want the crates whose *max* version is dependent, so we join on a
        -- subselect that includes the versions with their ordinal position
        (
            SELECT DISTINCT ON (crate_id)
               crate_id, semver_no_prerelease, id
            FROM versions
            WHERE NOT yanked
            ORDER BY
                crate_id,
                semver_no_prerelease DESC NULLS LAST,
                id DESC
        ) versions
        INNER JOIN crates
          ON crates.id = versions.crate_id
        WHERE versions.id IN (SELECT version_id FROM dependencies WHERE crate_id = $1)
    ) c
    ORDER BY
        crate_downloads DESC,
        crate_name ASC
) crates
-- Multiple dependencies can exist, we only want first one
CROSS JOIN LATERAL (
    SELECT dependencies.*
    FROM dependencies
    WHERE dependencies.crate_id = $1 AND dependencies.version_id = crates.version_id
    ORDER BY id ASC
    LIMIT 1
) dependencies
OFFSET $2
LIMIT $3
