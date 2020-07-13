CREATE INDEX api_tokens_token_idx ON api_tokens (token);
ALTER TABLE api_tokens ADD CONSTRAINT api_tokens_token_key UNIQUE (token);
