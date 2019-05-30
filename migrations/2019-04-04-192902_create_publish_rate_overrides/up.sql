CREATE TABLE publish_rate_overrides (
  user_id INTEGER PRIMARY KEY REFERENCES users,
  burst INTEGER NOT NULL
);
