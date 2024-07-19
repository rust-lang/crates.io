CREATE OR REPLACE aggregate tsvector_agg (tsvector) (
  STYPE = pg_catalog.tsvector,
  SFUNC = pg_catalog.tsvector_concat,
  INITCOND = ''
);

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
