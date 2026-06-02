-- Add `semver_ord(num)` function to convert a semver string into a JSONB array for version comparison purposes.

create or replace function semver_ord(num varchar) returns jsonb as $$
declare
    -- We need to ensure that the prerelease array has the same length for all
    -- versions since shorter arrays have lower precedence in JSONB. We store
    -- the first 10 parts of the prerelease string as pairs of booleans and
    -- numbers or text values, and then a final text item for the remaining
    -- parts.
    max_prerelease_parts constant int := 10;

    -- We ignore the "build metadata" part of the semver string, since it has
    -- no impact on the version ordering.
    match_result text[] := regexp_match(num, '^(\d+).(\d+).(\d+)(?:-([0-9A-Za-z\-.]+))?');

    prerelease jsonb;
    prerelease_parts text[];
    prerelease_part text;
    i int := 0;
begin
    if match_result is null then
        return null;
    end if;

    if match_result[4] is null then
        -- A JSONB object has higher precedence than an array, and versions with
        -- prerelease specifiers should have lower precedence than those without.
        prerelease := json_build_object();
    else
        prerelease := to_jsonb(array_fill(NULL::bool, ARRAY[max_prerelease_parts * 2 + 1]));

        -- Split prerelease string by `.` and "append" items to
        -- the `prerelease` array.
        prerelease_parts := string_to_array(match_result[4], '.');

        foreach prerelease_part in array prerelease_parts[1:max_prerelease_parts + 1]
        loop
            -- Parse parts as numbers if they consist of only digits.
            if regexp_like(prerelease_part, '^\d+$') then
                -- In JSONB a number has higher precedence than a string but in
                -- semver it is the other way around, so we use true/false to
                -- work around this.
                prerelease := jsonb_set(prerelease, array[i::text], to_jsonb(false));
                prerelease := jsonb_set(prerelease, array[(i + 1)::text], to_jsonb(prerelease_part::numeric));
            else
                prerelease := jsonb_set(prerelease, array[i::text], to_jsonb(true));
                prerelease := jsonb_set(prerelease, array[(i + 1)::text], to_jsonb(prerelease_part));
            end if;

            -- Exit the loop if we have reached the maximum number of parts.
            i := i + 2;
            exit when i >= max_prerelease_parts * 2;
        end loop;

        prerelease := jsonb_set(prerelease, array[(max_prerelease_parts * 2)::text], to_jsonb(array_to_string(prerelease_parts[max_prerelease_parts + 1:], '.')));
    end if;

    -- Return an array with the major, minor, patch, and prerelease parts.
    return json_build_array(
        match_result[1]::numeric,
        match_result[2]::numeric,
        match_result[3]::numeric,
        prerelease
    );
end;
$$ language plpgsql immutable;

comment on function semver_ord is 'Converts a semver string into a JSONB array for version comparison purposes. The array has the following format: [major, minor, patch, prerelease] and when used for sorting follow the precedence rules defined in the semver specification (https://semver.org/#spec-item-11).';


-- Add corresponding column to the `versions` table.

alter table versions
    add semver_ord jsonb;

comment on column versions.semver_ord is 'JSONB representation of the version number for sorting purposes.';


-- Create a trigger to set the `semver_ord` column when inserting a new version.
-- Ideally, we would use a generated column for this, but introducing such a
-- column would require a full table rewrite, which is not feasible for large
-- tables.

create or replace function set_semver_ord() returns trigger as $$
begin
    new.semver_ord := semver_ord(new.num);
    return new;
end
$$ language plpgsql;

create or replace trigger trigger_set_semver_ord
    before insert on versions
    for each row
    execute procedure set_semver_ord();


-- Populate the `semver_ord` column for existing versions.
-- This query should be run manually in small batches to avoid locking the
-- table for too long.

-- with versions_to_update as (
--     select id, num
--     from versions
--     where semver_ord = 'null'::jsonb
--     limit 1000
-- )
-- update versions
--     set semver_ord = semver_ord(num)
--     where id in (select id from versions_to_update);
