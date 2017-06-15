CREATE TABLE api_tokens (
  id SERIAL PRIMARY KEY,
  user_id integer NOT NULL REFERENCES users(id),
  token character varying DEFAULT random_string(32) NOT NULL UNIQUE,
  name character varying NOT NULL,
  created_at timestamp without time zone DEFAULT now() NOT NULL,
  last_used_at timestamp without time zone
);

CREATE INDEX ON api_tokens (token);

INSERT INTO api_tokens (user_id, token, name)
  SELECT id, api_token, 'Initial token' FROM users;

-- To be done in a cleanup migration later.
-- ALTER TABLE users DROP COLUMN api_token;
