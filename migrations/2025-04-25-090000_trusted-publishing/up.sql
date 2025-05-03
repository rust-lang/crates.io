create table trustpub_configs_github
(
    id serial primary key,
    created_at timestamptz not null default now(),
    crate_id int not null references crates on delete cascade,
    repository_owner varchar not null,
    repository_owner_id int not null,
    repository_name varchar not null,
    workflow_filename varchar not null,
    environment varchar
);

comment on table trustpub_configs_github is 'Trusted Publisher configuration for GitHub Actions';
comment on column trustpub_configs_github.id is 'Unique identifier of the `trustpub_configs_github` row';
comment on column trustpub_configs_github.created_at is 'Date and time when the configuration was created';
comment on column trustpub_configs_github.crate_id is 'Unique identifier of the crate that this configuration is for';
comment on column trustpub_configs_github.repository_owner is 'GitHub name of the user or organization that owns the repository';
comment on column trustpub_configs_github.repository_owner_id is 'GitHub ID of the user or organization that owns the repository';
comment on column trustpub_configs_github.repository_name is 'Name of the repository that this configuration is for';
comment on column trustpub_configs_github.workflow_filename is 'Name of the workflow file inside the repository that will be used to publish the crate';
comment on column trustpub_configs_github.environment is 'GitHub Actions environment that will be used to publish the crate (if `NULL` the environment is unrestricted)';

-------------------------------------------------------------------------------

create table trustpub_tokens
(
    id bigserial primary key,
    created_at timestamptz not null default now(),
    expires_at timestamptz not null,
    hashed_token bytea not null,
    crate_ids int[] not null
);

comment on table trustpub_tokens is 'Temporary access tokens for Trusted Publishing';
comment on column trustpub_tokens.id is 'Unique identifier of the `trustpub_tokens` row';
comment on column trustpub_tokens.created_at is 'Date and time when the token was created';
comment on column trustpub_tokens.expires_at is 'Date and time when the token will expire';
comment on column trustpub_tokens.hashed_token is 'SHA256 hash of the token that can be used to publish the crate';
comment on column trustpub_tokens.crate_ids is 'Unique identifiers of the crates that can be published using this token';

create unique index trustpub_tokens_hashed_token_uindex
    on trustpub_tokens (hashed_token);

-------------------------------------------------------------------------------

create table trustpub_used_jtis
(
    id bigserial primary key,
    jti varchar not null,
    used_at timestamptz not null default now(),
    expires_at timestamptz not null
);

comment on table trustpub_used_jtis is 'Used JWT IDs to prevent token reuse in the Trusted Publishing flow';
comment on column trustpub_used_jtis.id is 'Unique identifier of the `trustpub_used_jtis` row';
comment on column trustpub_used_jtis.jti is 'JWT ID from the OIDC token';
comment on column trustpub_used_jtis.used_at is 'Date and time when the JWT was used';
comment on column trustpub_used_jtis.expires_at is 'Date and time when the JWT would expire';

create unique index trustpub_used_jtis_jti_uindex
    on trustpub_used_jtis (jti);
