drop trigger trigger_set_semver_ord_v2 on versions;
drop function set_semver_ord_v2();
alter table versions drop column semver_ord_v2;
drop function semver_ord_v2;
drop function semver_ord_num;
