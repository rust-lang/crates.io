alter table users
    add column gh_login VARCHAR NOT NULL;

CREATE INDEX lower_gh_login ON users (lower(gh_login));
