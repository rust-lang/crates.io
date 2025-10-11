-- Add a new column for identifying the primary email
ALTER TABLE emails ADD COLUMN is_primary BOOLEAN DEFAULT FALSE NOT NULL;
comment on column emails.is_primary is 'Whether this email is the primary email address for the user.';

-- After this migration has been applied, please run the following SQL to set the primary email for each user:
-- UPDATE emails SET is_primary = TRUE
