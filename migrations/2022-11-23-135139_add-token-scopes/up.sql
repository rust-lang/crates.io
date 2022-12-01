alter table api_tokens
    add column crate_scopes text[];

comment on column api_tokens.crate_scopes is 'NULL or an array of crate scope patterns (see RFC #2947)';

alter table api_tokens
    add column endpoint_scopes text[];

comment on column api_tokens.endpoint_scopes is 'An array of endpoint scopes or NULL for the `legacy` endpoint scope (see RFC #2947)';
