-- This is an aggregation function that combines multiple rows of tsvector data into a single tsvector
-- using the tsvector concat operator.
CREATE OR REPLACE aggregate tsvector_agg (tsvector) (
  STYPE = pg_catalog.tsvector,
  SFUNC = pg_catalog.tsvector_concat,
  INITCOND = ''
);
-- e.g.
-- WITH expected AS (
--   SELECT
--     'macro:1'::tsvector || 'any:1'::tsvector AS concat
-- ),
-- data as (
--   SELECT *
--   FROM (
--     VALUES
--       ('macro:1' :: tsvector),
--       ('any:1' :: tsvector)
--   ) k(tv)
-- )
-- SELECT
--   ( SELECT concat FROM expected ),
--   ( SELECT tsvector_agg(tv) FROM data ) AS agg,
--   ( SELECT concat FROM expected ) = (
--     SELECT tsvector_agg(tv) FROM data
--   ) AS is_eq;
--
-- EOF
--       concat       |        agg        | is_eq
-- -------------------+-------------------+-------
--  'any':2 'macro':1 | 'any':2 'macro':1 | t
-- (1 row)

-- Add support for storing keywords considered stopwords in `crates.textsearchable_index_col` by casting
-- to tsvector
CREATE OR REPLACE FUNCTION trigger_crates_name_search() RETURNS trigger AS $$
DECLARE kws tsvector;
begin
  SELECT
    tsvector_agg(
      CASE WHEN length(to_tsvector('english', keyword)) != 0 THEN to_tsvector('english', keyword)
      ELSE (keyword || ':1')::tsvector
      END
      ORDER BY keyword
  	) INTO kws
  FROM keywords INNER JOIN crates_keywords
    ON keywords.id = crates_keywords.keyword_id
  WHERE crates_keywords.crate_id = new.id;
    new.textsearchable_index_col :=
      setweight(to_tsvector('pg_catalog.english', coalesce(new.name, '')), 'A') ||
      setweight(kws, 'B') ||
      setweight(to_tsvector('pg_catalog.english', coalesce(new.description, '')), 'C') ||
      setweight(to_tsvector('pg_catalog.english', coalesce(new.readme, '')), 'D')
  ;
  return new;
end
$$ LANGUAGE plpgsql;


-- We could update those crates with the following sql
--
-- WITH keywords_with_stopwords as (
--   SELECT crate_id, keyword
--   FROM keywords INNER JOIN crates_keywords
--     ON id = keyword_id
--   WHERE length(to_tsvector('english', keyword)) = 0
-- )
-- UPDATE crates
-- SET updated_at = updated_at
-- FROM keywords_with_stopwords
-- WHERE id = crate_id AND NOT (keyword || ':B')::tsquery @@ textsearchable_index_col
-- ;
