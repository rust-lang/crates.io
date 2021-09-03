create or replace function to_semver_no_prerelease(text) returns semver_triple
    immutable
    language sql
as
$$
SELECT (
        split_part($1, '.', 1)::numeric,
        split_part($1, '.', 2)::numeric,
        split_part(split_part($1, '+', 1), '.', 3)::numeric
           )::semver_triple
WHERE strpos($1, '-') = 0
$$;
