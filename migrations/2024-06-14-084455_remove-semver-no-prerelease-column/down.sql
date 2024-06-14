create function to_semver_no_prerelease(text) returns semver_triple
    immutable
    parallel safe
    language sql
as
$$
  SELECT (
    split_part($1, '.', 1)::numeric,
    split_part($1, '.', 2)::numeric,
    split_part(split_part($1, '+', 1), '.', 3)::numeric
  )::semver_triple
  WHERE strpos(split_part($1, '+', 1), '-') = 0
  $$;

alter table versions
    add semver_no_prerelease semver_triple generated always as (to_semver_no_prerelease(num)) stored;
