-- Reset all ID column sequences to their maximum values or 1
-- This script dynamically discovers all sequences associated with 'id' columns
-- and sets them to either the maximum id value in the table or 1 if empty

DO $$
DECLARE
    rec RECORD;
    max_id BIGINT;
BEGIN
    FOR rec IN
        WITH id_sequences AS (
            SELECT
                pg_class.relname AS sequence_name,
                pg_class_tables.relname AS table_name
            FROM pg_class
            JOIN pg_depend ON pg_depend.objid = pg_class.oid
            JOIN pg_class pg_class_tables ON pg_depend.refobjid = pg_class_tables.oid
            JOIN pg_attribute ON pg_depend.refobjid = pg_attribute.attrelid AND pg_depend.refobjsubid = pg_attribute.attnum
            JOIN pg_namespace ON pg_class_tables.relnamespace = pg_namespace.oid
            WHERE pg_class.relkind = 'S' -- sequences
            AND pg_attribute.attname = 'id'  -- only 'id' columns
            AND pg_namespace.nspname = 'public' -- only public schema
        )
        SELECT * FROM id_sequences
    LOOP
        -- Get the maximum id value from the table
        EXECUTE format('SELECT MAX(id) FROM public.%I', rec.table_name) INTO max_id;

        -- Reset the sequence to the max value or 1 if empty
        --
        -- Use is_called = false for empty tables (max_id IS NULL),
        -- and is_called = true for populated tables
        PERFORM setval('public.' || rec.sequence_name, COALESCE(max_id, 1), max_id IS NOT NULL);

        -- Log the action
        RAISE NOTICE 'Reset sequence % to % (table: %, is_called: %)', rec.sequence_name, COALESCE(max_id, 1), rec.table_name, max_id IS NOT NULL;
    END LOOP;
END $$;
