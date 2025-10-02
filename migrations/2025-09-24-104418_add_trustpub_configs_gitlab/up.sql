CREATE TABLE trustpub_configs_gitlab (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    crate_id INTEGER NOT NULL REFERENCES crates ON DELETE CASCADE,
    namespace VARCHAR NOT NULL,
    namespace_id VARCHAR,
    project VARCHAR NOT NULL,
    workflow_filepath VARCHAR NOT NULL,
    environment VARCHAR
);

comment on table trustpub_configs_gitlab is 'Trusted Publisher configuration for GitLab CI';
comment on column trustpub_configs_gitlab.id is 'Unique identifier of the `trustpub_configs_gitlab` row';
comment on column trustpub_configs_gitlab.created_at is 'Date and time when the configuration was created';
comment on column trustpub_configs_gitlab.crate_id is 'Unique identifier of the crate that this configuration is for';
comment on column trustpub_configs_gitlab.namespace is 'GitLab namespace (user or group) that owns the project';
comment on column trustpub_configs_gitlab.namespace_id is 'GitLab namespace ID, populated on first token exchange for resurrection attack protection';
comment on column trustpub_configs_gitlab.project is 'Name of the GitLab project that this configuration is for';
comment on column trustpub_configs_gitlab.workflow_filepath is 'Path to the CI/CD configuration file that will be used to publish the crate';
comment on column trustpub_configs_gitlab.environment is 'GitLab environment that will be used to publish the crate (if `NULL` the environment is unrestricted)';
