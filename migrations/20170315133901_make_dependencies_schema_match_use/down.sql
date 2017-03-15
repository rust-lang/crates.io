ALTER TABLE dependencies
  ALTER COLUMN kind DROP DEFAULT,
  ALTER COLUMN kind DROP NOT NULL,
  ALTER COLUMN features SET DATA TYPE text USING array_to_string(features, ',');
