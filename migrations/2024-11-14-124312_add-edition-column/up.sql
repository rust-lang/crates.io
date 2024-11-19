alter table versions
    add column edition text;

comment on column versions.edition is 'The declared Rust Edition required to compile this version of the crate.';
