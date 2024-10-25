create unique index concurrently if not exists versions_crate_id_num_no_build_uindex
    on versions (crate_id, num_no_build);
