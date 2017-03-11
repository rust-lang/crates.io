CREATE INDEX versions_to_semver_no_prerelease_idx ON versions (crate_id, to_semver_no_prerelease(num) DESC NULLS LAST) WHERE NOT yanked;
