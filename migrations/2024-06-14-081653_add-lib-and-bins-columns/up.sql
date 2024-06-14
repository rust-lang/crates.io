alter table versions
    add has_lib boolean,
    add bin_names text[];

comment on column versions.has_lib is 'TRUE if the version has a library (e.g. `src/lib.rs`), FALSE if no library was detected, or NULL if the version has not been analyzed yet.';
comment on column versions.bin_names is 'list of the names of all detected binaries in the version. the list may be empty which indicates that no binaries were detected in the version. the column may be NULL is the version has not been analyzed yet.';
