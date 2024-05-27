create index concurrently if not exists versions_id_yanked_idx
    on versions (id) WHERE yanked;
