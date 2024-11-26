alter table versions
    add column description text,
    add column homepage text,
    add column documentation text,
    add column repository text;

comment on column versions.description is 'Value of the `description` field in the `Cargo.toml` file of this version.';
comment on column versions.homepage is 'Value of the `homepage` field in the `Cargo.toml` file of this version.';
comment on column versions.documentation is 'Value of the `documentation` field in the `Cargo.toml` file of this version.';
comment on column versions.repository is 'Value of the `repository` field in the `Cargo.toml` file of this version.';
