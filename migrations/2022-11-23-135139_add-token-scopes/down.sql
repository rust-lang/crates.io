alter table api_tokens
    drop column crate_scopes;

alter table api_tokens
    drop column endpoint_scopes;
