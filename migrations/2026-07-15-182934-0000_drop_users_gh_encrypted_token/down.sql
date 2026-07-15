alter table users
    add column gh_encrypted_token bytea;

comment on column users.gh_encrypted_token is 'Encrypted GitHub access token';
