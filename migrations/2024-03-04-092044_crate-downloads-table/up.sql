-- Create the `crate_downloads` table.

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

-- Create a trigger to automatically add a row to `crate_downloads` when a new
-- crate is inserted into the `crates` table.

create or replace function insert_crate_downloads_row() returns trigger as $$
begin
    insert into crate_downloads(crate_id) values (new.id);
    return new;
end;
$$ language plpgsql;

create trigger insert_crate_downloads_row
    after insert on crates
    for each row
execute function insert_crate_downloads_row();

-- The following query can take a couple of seconds so it should be run manually
-- outside of the migration to prevent the server from taking a long time to
-- start up while waiting for the migration to complete.

-- insert into crate_downloads (crate_id, downloads)
-- select id, downloads
-- from crates;
