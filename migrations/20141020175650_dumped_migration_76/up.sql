CREATE FUNCTION trigger_crates_name_search() RETURNS trigger AS $$
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

            CREATE TRIGGER trigger_crates_tsvector_update BEFORE INSERT OR UPDATE
            ON crates
            FOR EACH ROW EXECUTE PROCEDURE trigger_crates_name_search();