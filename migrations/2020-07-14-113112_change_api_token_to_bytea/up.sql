CREATE EXTENSION IF NOT EXISTS pgcrypto;
ALTER TABLE api_tokens
  ALTER COLUMN token
  TYPE bytea USING decode(token, 'escape');
CREATE UNIQUE INDEX ON api_tokens (token);
