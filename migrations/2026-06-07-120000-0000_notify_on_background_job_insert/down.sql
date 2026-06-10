DROP TRIGGER IF EXISTS background_jobs_notify_on_insert ON background_jobs;
DROP FUNCTION IF EXISTS notify_background_job_inserted CASCADE;
