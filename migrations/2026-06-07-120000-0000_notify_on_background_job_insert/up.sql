-- Send a `NOTIFY` on the `background_jobs` channel whenever new jobs are
-- inserted, so that idle background workers can wake up immediately instead of
-- waiting for their next poll. This is a statement-level trigger, so batch
-- inserts only emit a single notification. The notification is delivered when
-- the surrounding transaction commits, i.e. once the new jobs are visible.

CREATE OR REPLACE FUNCTION notify_background_job_inserted() RETURNS TRIGGER AS $$
BEGIN
    NOTIFY background_jobs;
    RETURN NULL;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS background_jobs_notify_on_insert ON background_jobs;
CREATE TRIGGER background_jobs_notify_on_insert
    AFTER INSERT ON background_jobs
    FOR EACH STATEMENT
    EXECUTE PROCEDURE notify_background_job_inserted();
