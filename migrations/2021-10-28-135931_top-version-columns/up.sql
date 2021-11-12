ALTER TABLE crates ADD COLUMN highest_version VARCHAR;
ALTER TABLE crates ADD COLUMN highest_stable_version VARCHAR;
ALTER TABLE crates ADD COLUMN newest_version VARCHAR;

COMMENT ON COLUMN crates.highest_version IS 'The "highest" version in terms of semver';
COMMENT ON COLUMN crates.highest_stable_version IS 'The "highest" non-prerelease version in terms of semver';
COMMENT ON COLUMN crates.newest_version IS 'The "newest" version in terms of publishing date';
