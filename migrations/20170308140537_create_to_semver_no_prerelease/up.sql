CREATE TYPE semver_triple AS (
  major int4,
  minor int4,
  teeny int4
);

CREATE FUNCTION to_semver_no_prerelease(text) RETURNS semver_triple IMMUTABLE AS $$
  SELECT (
    split_part($1, '.', 1)::int4,
    split_part($1, '.', 2)::int4,
    split_part(split_part($1, '+', 1), '.', 3)::int4
  )::semver_triple
  WHERE strpos($1, '-') = 0
  $$ LANGUAGE SQL
;

CREATE INDEX versions_to_semver_no_prerelease_idx ON versions (crate_id, to_semver_no_prerelease(num) DESC NULLS LAST) WHERE NOT yanked;
