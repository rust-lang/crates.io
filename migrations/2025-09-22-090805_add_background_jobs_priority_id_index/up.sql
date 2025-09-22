CREATE INDEX CONCURRENTLY background_jobs_priority_id_index
    ON background_jobs (priority DESC, id ASC);
