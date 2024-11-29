alter table versions
    add column categories text[] default array[]::text[] not null,
    add column keywords text[] default array[]::text[] not null;

comment on column versions.categories is 'The list of `categories` in the `Cargo.toml` file of this version.';
comment on column versions.keywords is 'The list of `keywords` in the `Cargo.toml` file of this version.';
