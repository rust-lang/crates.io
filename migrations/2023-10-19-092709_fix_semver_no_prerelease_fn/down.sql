CREATE OR REPLACE FUNCTION to_semver_no_prerelease(text) RETURNS public.semver_triple IMMUTABLE PARALLEL SAFE AS $$
  SELECT (
    split_part($1, '.', 1)::numeric,
    split_part($1, '.', 2)::numeric,
    split_part(split_part($1, '+', 1), '.', 3)::numeric
  )::public.semver_triple
  WHERE strpos($1, '-') = 0
  $$ LANGUAGE SQL;
