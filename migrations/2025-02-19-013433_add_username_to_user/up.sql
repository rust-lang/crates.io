ALTER TABLE users
-- The column needs to be nullable for this migration to be fast; can be changed to non-nullable
-- after backfill of all records.
ADD COLUMN username VARCHAR;

CREATE INDEX lower_username ON users (lower(username));
