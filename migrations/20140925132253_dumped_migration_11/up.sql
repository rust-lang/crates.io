UPDATE versions SET updated_at = now() WHERE updated_at IS NULL;
UPDATE versions SET created_at = now() WHERE created_at IS NULL;