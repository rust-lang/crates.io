create or replace function to_semver_no_prerelease(text) returns semver_triple
    immutable
    language sql
as
$$
SELECT (
        -- 2) then, we extract the major, minor and patch numbers
        --    (dropping the prerelease part to avoid number conversion errors)
        split_part(version_without_metadata, '.', 1)::numeric,
        split_part(version_without_metadata, '.', 2)::numeric,
        split_part(split_part(version_without_metadata, '-', 1), '.', 3)::numeric
           )::semver_triple
FROM (
    -- 1) first, we split off the release metadata, if it exists
    SELECT split_part($1, '+', 1) as version_without_metadata
    ) as vwm
-- 3) finally, we only return the result if there is no prerelease part
WHERE strpos(version_without_metadata, '-') = 0
$$;
