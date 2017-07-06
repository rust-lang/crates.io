SELECT crates.name, sum_downloads 
FROM (
    SELECT crate_id, SUM(downloads) as sum_downloads 
    FROM crate_downloads 
    WHERE date > CURRENT_DATE - INTERVAL '90 days' 
    GROUP BY crate_id
    ) as downloads 
INNER JOIN crates ON crates.id = downloads.crate_id 
ORDER BY sum_downloads DESC;
