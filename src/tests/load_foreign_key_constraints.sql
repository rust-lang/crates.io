-- This can't quite use Diesel's query builder yet, because diesel doesn't
-- have the `ARRAY[]` constructor
SELECT relname, conname, pg_get_constraintdef(pg_constraint.oid, true) AS definition
    FROM pg_attribute
    INNER JOIN pg_class ON pg_class.oid = attrelid
    LEFT JOIN pg_constraint ON pg_class.oid = conrelid AND ARRAY[attnum] = conkey
    WHERE attname = $1
      AND contype = 'f'
