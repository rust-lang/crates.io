alter table users
    add column gh_id INTEGER;

comment on column users.gh_id is 'Associated GitHub ID';
