alter table versions
    add column lib_name text;

comment on column versions.lib_name is 'The library target name for this version (the Rust identifier used in `use` statements), or NULL if the version has no library target or has not been analyzed yet.';
