UPDATE dependencies SET kind = 0 WHERE kind IS NULL;
ALTER TABLE dependencies
  ALTER COLUMN kind SET DEFAULT 0,
  ALTER COLUMN kind SET NOT NULL,
  ALTER COLUMN features SET DATA TYPE text[] USING string_to_array(features, ',');
