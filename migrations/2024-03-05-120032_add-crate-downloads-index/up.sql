create index concurrently if not exists crate_downloads_downloads_crate_id_index
    on crate_downloads (downloads desc, crate_id desc);
