drop trigger if exists trigger_set_semver_ord on versions;
drop function if exists set_semver_ord();
-- safety-assured:start
-- The column is no longer used.
alter table versions drop column if exists semver_ord;
-- safety-assured:end
drop function if exists semver_ord(varchar);
