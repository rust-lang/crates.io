alter table versions
    add num_no_build varchar;

comment on column versions.num_no_build is 'This is the same as `num` without the optional "build metadata" part (except for some versions that were published before we started validating this).';

-- to be run manually:

-- update versions
--     set num_no_build = split_part(num, '+', 1);
--
-- with duplicates as (
--     -- find all versions that have the same `crate_id` and `num_no_build`
--     select crate_id, num_no_build, array_agg(num ORDER BY id) as nums
--     from versions
--     group by crate_id, num_no_build
--     having count(*) > 1
-- ),
-- duplicates_to_update as (
--     -- for each group of duplicates, update all versions except the one that
--     -- doesn't have "build metadata", or the first one that was published if
--     -- all versions have "build metadata"
--     select crate_id, num_no_build, unnest(case when array_position(nums, num_no_build) IS NOT NULL then array_remove(nums, num_no_build) else nums[2:] end) as num
--     from duplicates
-- )
-- update versions
--     set num_no_build = duplicates_to_update.num
--     from duplicates_to_update
--     where versions.crate_id = duplicates_to_update.crate_id
--     and versions.num = duplicates_to_update.num;
