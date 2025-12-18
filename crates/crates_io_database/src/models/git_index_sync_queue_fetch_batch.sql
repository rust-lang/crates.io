WITH
    batch AS (
        SELECT
            crate_name
        FROM
            git_index_sync_queue
        ORDER BY
            created_at ASC
        FOR UPDATE
        LIMIT
            $1
    )
DELETE FROM git_index_sync_queue USING batch
WHERE
    git_index_sync_queue.crate_name = batch.crate_name
RETURNING
    git_index_sync_queue.crate_name,
    git_index_sync_queue.created_at;
