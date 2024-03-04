create table crate_downloads
(
    crate_id  integer          not null
        constraint crate_downloads_pk
            primary key
        constraint crate_downloads_crates_id_fk
            references crates
            on delete cascade,
    downloads bigint default 0 not null
);

comment on table crate_downloads is 'Number of downloads per crate. This was extracted from the `crates` table for performance reasons.';
comment on column crate_downloads.crate_id is 'Reference to the crate that this row belongs to.';
comment on column crate_downloads.downloads is 'The total number of downloads for this crate.';

-- The following query can take a couple of seconds so it should be run manually
-- outside of the migration to prevent the server from taking a long time to
-- start up while waiting for the migration to complete.

-- insert into crate_downloads (crate_id, downloads)
-- select id, downloads
-- from crates;
