create table default_versions
(
    crate_id   integer not null
        constraint default_versions_pk
            primary key
        constraint default_versions_crates_id_fk
            references crates
            on delete cascade,
    version_id integer not null
        constraint default_versions_versions_id_fk
            references versions
            on delete no action deferrable initially deferred
);

create unique index default_versions_version_id_uindex
    on default_versions (version_id);

comment on table default_versions is 'A mapping from crates to the versions that the frontend will display by default.';
comment on column default_versions.crate_id is 'Reference to the crate in the `crates` table.';
comment on column default_versions.version_id is 'Reference to the version in the `versions` table.';
comment on constraint default_versions_crates_id_fk on default_versions is
    'This is a `cascade` delete because we want to delete the row when the crate is deleted.';
comment on constraint default_versions_versions_id_fk on default_versions is
    'This is a `no action` delete because we want to fail the version deletion unless the default version is updated in the same transaction.';
