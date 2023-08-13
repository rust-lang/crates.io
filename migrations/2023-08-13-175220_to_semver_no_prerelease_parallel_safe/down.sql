DROP FUNCTION to_semver_no_prerelease(text);

CREATE FUNCTION to_semver_no_prerelease(text) RETURNS semver_triple IMMUTABLE AS $$
  SELECT (
    split_part($1, '.', 1)::numeric,
    split_part($1, '.', 2)::numeric,
    split_part(split_part($1, '+', 1), '.', 3)::numeric
  )::semver_triple
  WHERE strpos($1, '-') = 0
  $$ LANGUAGE SQL;
