CREATE TABLE linked_accounts (
  user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  provider INTEGER NOT NULL,
  account_id INTEGER NOT NULL,
  access_token VARCHAR NOT NULL,
  login VARCHAR NOT NULL,
  avatar VARCHAR,
  PRIMARY KEY (provider, account_id)
);
