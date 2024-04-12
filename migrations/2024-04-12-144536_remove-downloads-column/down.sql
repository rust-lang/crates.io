alter table crates
    add column downloads integer not null default 0;

create index concurrently index_crate_downloads
    on crates (downloads);

create index concurrently index_crates_id_downloads_name
    on crates (id, downloads, name);

-- The following query can take a couple of seconds so it should be run manually
-- outside of the migration to prevent the server from taking a long time to
-- start up while waiting for the migration to complete.

-- update crates
-- set downloads = (select downloads from crate_downloads where crate_downloads.crate_id = crates.id);
