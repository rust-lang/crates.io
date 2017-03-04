ALTER TABLE crates ADD COLUMN keywords VARCHAR;
                CREATE OR REPLACE FUNCTION trigger_crates_name_search() RETURNS trigger AS $$
                begin
                  new.textsearchable_index_col :=
                     setweight(to_tsvector('pg_catalog.english',
                                           coalesce(new.name, '')), 'A') ||
                     setweight(to_tsvector('pg_catalog.english',
                                           coalesce(new.keywords, '')), 'B') ||
                     setweight(to_tsvector('pg_catalog.english',
                                           coalesce(new.description, '')), 'C') ||
                     setweight(to_tsvector('pg_catalog.english',
                                           coalesce(new.readme, '')), 'D');
                  return new;
                end
                $$ LANGUAGE plpgsql;

                UPDATE crates SET keywords = (
                  SELECT array_to_string(array_agg(keyword), ',')
                    FROM keywords INNER JOIN crates_keywords
                    ON keywords.id = crates_keywords.keyword_id
                    WHERE crates_keywords.crate_id = crates.id
                );

                DROP TRIGGER touch_crate_on_modify_keywords ON crates_keywords;
                DROP FUNCTION touch_crate();