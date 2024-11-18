create table deleted_crates
(
    id           serial primary key,
    name         varchar     not null,
    created_at   timestamptz not null,
    deleted_at   timestamptz not null,
    deleted_by   integer
        constraint deleted_crates_users_id_fk
            references users
            on delete set null,
    message      varchar,
    available_at timestamptz not null
);

comment on table deleted_crates is 'Crates that have been deleted by users';
comment on column deleted_crates.id is 'Unique identifier of the `deleted_crates` row';
comment on column deleted_crates.name is 'Name of the deleted crate (use `canon_crate_name()` for normalization, if needed)';
comment on column deleted_crates.created_at is 'Date and time when the crate was created';
comment on column deleted_crates.deleted_at is 'Date and time when the crate was deleted';
comment on column deleted_crates.deleted_by is 'ID of the user who deleted the crate, or NULL if the user was deleted';
comment on column deleted_crates.message is 'Optional message left by the user who deleted the crate';
comment on column deleted_crates.available_at is 'Date and time when users will be able to create a new crate with the same name';

create index deleted_crates_canon_crate_name_index
    on deleted_crates (canon_crate_name(name));
