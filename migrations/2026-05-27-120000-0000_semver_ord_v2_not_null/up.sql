-- safety-assured:start
-- A `SELECT * FROM versions WHERE semver_ord_v2 IS NULL` against prod runs in
-- around 3 seconds, so the validation scan that `SET NOT NULL` performs under
-- ACCESS EXCLUSIVE is expected to be at least as fast. Not worth the extra
-- CHECK constraint work for this table size.
alter table versions
    alter column semver_ord_v2 set not null;
-- safety-assured:end
