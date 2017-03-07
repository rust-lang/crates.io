CREATE FUNCTION canon_crate_name(text) RETURNS text AS $$
                    SELECT replace(lower($1), '-', '_')
                $$ LANGUAGE SQL
            ;