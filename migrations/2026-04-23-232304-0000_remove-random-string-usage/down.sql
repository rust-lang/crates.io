ALTER TABLE emails
ALTER COLUMN token
SET DEFAULT random_string (26);
