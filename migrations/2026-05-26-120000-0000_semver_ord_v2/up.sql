-- Order-preserving binary (`bytea`) sort key for semver version strings.
--
-- `bytea` is compared byte-by-byte (memcmp), independent of the database
-- collation, so `ORDER BY semver_ord_v2(version)` matches semver precedence.
--
-- This is a replacement for the JSONB-based `semver_ord` function and column.
-- During the transition both representations coexist; the old column will be
-- dropped once all rows have been backfilled and all readers have switched
-- over.

-- Encode a semver numeric identifier as [digit_count][ASCII digits]. Byte order
-- == numeric order: with no leading zeros, more digits means a larger number so
-- the count byte dominates, and equal-length values compare digit-wise (ASCII
-- order == numeric order for same-length decimals). It is arbitrary-precision up
-- to 255 digits (the count byte's range).
create or replace function semver_ord_num(digits text) returns bytea as $$
    select set_byte('\x00'::bytea, 0, length(digits)) || convert_to(digits, 'UTF8');
$$ language 'sql' immutable parallel safe strict;

-- Layout: enc(major) enc(minor) enc(patch) <prerelease>, where enc() is the
-- length-prefixed key above.
--   no prerelease -> 0x03
--   prerelease    -> one tagged identifier per dot field, then a 0x00 end byte:
--     numeric      -> 0x01 enc(number)
--     alphanumeric -> 0x02 <raw ASCII bytes>
-- The single post-patch byte orders 0x00 (end) < 0x01 (numeric) < 0x02 (alpha)
-- < 0x03 (no prerelease), so "fewer fields < more fields", "numeric < alpha",
-- and "prerelease < release". Build metadata is ignored.
create or replace function semver_ord_v2(num text) returns bytea as $$
declare
    m text[] := regexp_match(
        num,
        '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-([0-9A-Za-z.-]+))?(?:\+.*)?$'
    );
    result bytea;
    parts text[];
    part text;
begin
    if m is null then
        return null;
    end if;

    result := semver_ord_num(m[1]) || semver_ord_num(m[2]) || semver_ord_num(m[3]);

    if m[4] is null then
        result := result || '\x03'::bytea;
    else
        parts := string_to_array(m[4], '.');
        foreach part in array parts loop
            if part ~ '^(0|[1-9][0-9]*)$' then
                result := result || '\x01'::bytea || semver_ord_num(part);
            else
                result := result || '\x02'::bytea || convert_to(part, 'UTF8');
            end if;
        end loop;
        result := result || '\x00'::bytea;
    end if;

    return result;
end;
$$ language 'plpgsql' immutable parallel safe strict;

comment on function semver_ord_v2 is 'Converts a semver string into an order-preserving bytea sort key. Byte-wise comparison of the result matches semver precedence (https://semver.org/#spec-item-11).';


-- Add corresponding column to the `versions` table.

alter table versions
    add column if not exists semver_ord_v2 bytea;

comment on column versions.semver_ord_v2 is 'Order-preserving bytea representation of the version number for sorting purposes.';


-- Create a trigger to set the `semver_ord_v2` column when inserting a new
-- version. Ideally, we would use a generated column for this, but introducing
-- such a column would require a full table rewrite, which is not feasible for
-- large tables.

create or replace function set_semver_ord_v2() returns trigger as $$
begin
    new.semver_ord_v2 := semver_ord_v2(new.num);
    return new;
end
$$ language 'plpgsql';

create or replace trigger trigger_set_semver_ord_v2
    before insert on versions
    for each row
    execute procedure set_semver_ord_v2();


-- Populate the `semver_ord_v2` column for existing versions.
-- This query should be run manually in small batches to avoid locking the
-- table for too long.

-- with versions_to_update as (
--     select id, num
--     from versions
--     where semver_ord_v2 is null
--     limit 1000
-- )
-- update versions
--     set semver_ord_v2 = semver_ord_v2(num)
--     where id in (select id from versions_to_update);
