ALTER TABLE api_tokens
  ALTER COLUMN token
  TYPE text USING encode(token, 'escape');
DROP INDEX api_tokens_token_idx;
