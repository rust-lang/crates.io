WITH downloads_batch AS (
    -- Select a batch of downloads (incl. `crate_id`) that have not been
    -- counted yet.
    SELECT crate_id, version_id, date, version_downloads.downloads - counted AS downloads
    FROM version_downloads
    INNER JOIN versions ON versions.id = version_id
    WHERE NOT processed AND version_downloads.downloads != counted
    LIMIT $1
), version_downloads_batch AS (
    -- Group the downloads by `version_id` and sum them up for the
    -- `updated_versions` CTE.
    SELECT version_id, SUM(downloads_batch.downloads) as downloads
    FROM downloads_batch
    GROUP BY version_id
), updated_versions AS (
    -- Update the `downloads` count for each version.
    UPDATE versions
    SET downloads = versions.downloads + version_downloads_batch.downloads
    FROM version_downloads_batch
    WHERE versions.id = version_downloads_batch.version_id
), crate_downloads_batch AS (
    -- Group the downloads by `crate_id` and sum them up for the
    -- `updated_crates` CTE.
    SELECT crate_id, SUM(downloads_batch.downloads) as downloads
    FROM downloads_batch
    GROUP BY crate_id
), updated_crate_downloads AS (
    -- Update the `downloads` count for each crate in the `crate_downloads` table.
    UPDATE crate_downloads
    SET downloads = crate_downloads.downloads + crate_downloads_batch.downloads
    FROM crate_downloads_batch
    WHERE crate_downloads.crate_id = crate_downloads_batch.crate_id
), updated_metadata AS (
    -- Update the `total_downloads` count in the `metadata` table.
    UPDATE metadata
    SET total_downloads = metadata.total_downloads + sum.downloads
    FROM (
        SELECT COALESCE(SUM(downloads), 0) as downloads
        FROM downloads_batch
    ) sum
    WHERE sum.downloads > 0
), sorted_downloads_batch AS (
    -- Sort the `downloads_batch` CTE by `version_id` and `date` to
    -- ensure that the `version_downloads` table is updated in a
    -- consistent order to avoid deadlocks.
    SELECT downloads_batch.*
    FROM version_downloads
    JOIN downloads_batch using (version_id, date)
    ORDER BY version_id, date
    FOR UPDATE
), updated_version_downloads AS (
    -- Update the `counted` value for each version in the batch.
    UPDATE version_downloads
    SET counted = version_downloads.counted + sorted_downloads_batch.downloads
    FROM sorted_downloads_batch
    WHERE version_downloads.version_id = sorted_downloads_batch.version_id
        AND version_downloads.date = sorted_downloads_batch.date
)
-- Return the number of rows in the `downloads_batch` CTE to determine whether
-- there are more rows to process.
SELECT COUNT(*) AS count FROM downloads_batch
