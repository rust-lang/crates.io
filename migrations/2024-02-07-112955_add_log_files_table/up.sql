create table processed_log_files
(
    path   varchar not null,
    time   timestamptz not null default now(),
    constraint processed_log_files_pk
        primary key (path)
);

comment on table processed_log_files is 'List of all processed CDN log files, used to avoid processing the same file multiple times.';

comment on column processed_log_files.path is 'Path of the log file inside the S3 bucket';
comment on column processed_log_files.time is 'Time when the log file was processed';
