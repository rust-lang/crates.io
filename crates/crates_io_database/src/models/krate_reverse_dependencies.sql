-- Serve a page of reverse dependencies for crate $1 from the precomputed
-- `reverse_dependencies` summary table.

SELECT
    dependencies.*,
    reverse_dependencies.dependent_downloads AS crate_downloads,
    crates.name AS crate_name,
    (
        SELECT COUNT(*)
        FROM reverse_dependencies
        WHERE target_crate_id = $1
    ) AS total
FROM reverse_dependencies
INNER JOIN crates
    ON crates.id = reverse_dependencies.dependent_crate_id
INNER JOIN dependencies
    ON dependencies.id = reverse_dependencies.dependency_id
WHERE reverse_dependencies.target_crate_id = $1
ORDER BY
    reverse_dependencies.dependent_downloads DESC,
    reverse_dependencies.dependent_crate_id DESC
OFFSET $2
LIMIT $3
