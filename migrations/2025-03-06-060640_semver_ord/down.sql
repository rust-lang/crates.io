drop trigger trigger_set_semver_ord on versions;
drop function set_semver_ord();
alter table versions drop column semver_ord;
drop function semver_ord;
