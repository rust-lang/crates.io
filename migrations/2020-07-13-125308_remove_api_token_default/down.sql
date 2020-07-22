ALTER TABLE api_tokens ALTER COLUMN token SET DEFAULT random_string(32);
