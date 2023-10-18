COMMENT ON COLUMN crate_owners.owner_kind IS '`owner_kind = 0` refers to `users`, `owner_kind = 1` refers to `teams`.';
COMMENT ON COLUMN crate_owners.owner_id IS 'This refers either to the `users.id` or `teams.id` column, depending on the value of the `owner_kind` column';
COMMENT ON COLUMN teams.login IS 'Example: `github:foo:bar` means the `bar` team of the `foo` GitHub organization.';
COMMENT ON COLUMN teams.github_id IS 'Unique team ID on the GitHub API. When teams are recreated with the same name then they will still get a different ID, so this allows us to avoid potential name reuse attacks.';
COMMENT ON COLUMN teams.org_id IS 'Unique organization ID on the GitHub API. When organizations are recreated with the same name then they will still get a different ID, so this allows us to avoid potential name reuse attacks.';
