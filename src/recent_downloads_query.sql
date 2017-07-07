SELECT
    id,
    name,
    updated_at,
    created_at,
    downloads,
    description,
    homepage,
    documentation,
    readme,
    license,
    repository,
    max_upload_size,
    sum_downloads,
    FALSE
FROM (
    SELECT crate_id, SUM(downloads) as sum_downloads 
    FROM crate_downloads 
    WHERE date > CURRENT_DATE - INTERVAL '90 days' 
    GROUP BY crate_id
    ) as downloads 
INNER JOIN crates ON crates.id = downloads.crate_id 
ORDER BY sum_downloads DESC
OFFSET $1
LIMIT $2
