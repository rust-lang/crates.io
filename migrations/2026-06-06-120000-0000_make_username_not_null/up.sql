
alter table users add constraint username_not_null check (username is not null) not valid;


alter table users validate constraint username_not_null;


alter table users alter column username set not null;


alter table users drop constraint username_not_null;

comment on column users.username is 'Username associated with the user''s crates.io account, independent of linked OAuth usernames. This column is now non-nullable following a data backfill from gh_login.';
