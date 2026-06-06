alter table users
    alter column username set not null;

comment on column users.username is 'Username associated with the user''s crates.io account, independent of linked OAuth usernames. This column is now non-nullable following a data backfill from gh_login.';
