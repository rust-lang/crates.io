-- This can't quite use Diesel's query builder yet, because diesel doesn't
-- have the `ARRAY[]` constructor
SELECT relname, conname, pg_get_constraintdef(pg_constraint.oid, true) AS definition
    FROM pg_attribute
    INNER JOIN pg_class ON pg_class.oid = attrelid
    LEFT JOIN pg_constraint ON pg_class.oid = conrelid AND ARRAY[attnum] = conkey
    INNER JOIN pg_namespace ON pg_namespace.oid = pg_class.relnamespace
    WHERE attname = $1
      AND relkind = 'r'
      AND (contype IS NULL OR contype = 'f')
      AND nspname = 'public';
