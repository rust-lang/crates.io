UPDATE packages SET updated_at = now() WHERE updated_at IS NULL;
UPDATE packages SET created_at = now() WHERE created_at IS NULL;