-- safety-assured:start
-- All rows in the `users` table already have a non-null `username` value after
-- the admin backfill from `gh_login`. The validation scan that `SET NOT NULL`
-- performs under ACCESS EXCLUSIVE is expected to be fast.
alter table users
    alter column username set not null;
-- safety-assured:end
