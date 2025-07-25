-- Remove the function for marking an email as primary
DROP FUNCTION mark_email_as_primary;

-- Remove the function that enforces the maximum number of emails per user
DROP TRIGGER trigger_enforce_max_emails_per_user ON emails;
DROP FUNCTION enforce_max_emails_per_user();

-- Remove the unique constraint for the combination of user_id and email
ALTER TABLE emails DROP CONSTRAINT unique_user_email;

-- Remove the constraint that allows only one primary email per user
ALTER TABLE emails DROP CONSTRAINT unique_primary_email_per_user;

-- Remove the trigger that enforces at least one primary email per user
DROP TRIGGER trigger_ensure_at_least_one_primary_email ON emails;
DROP FUNCTION ensure_at_least_one_primary_email();

-- Remove the trigger that prevents deletion of primary emails
DROP TRIGGER trigger_prevent_primary_email_deletion ON emails;
DROP FUNCTION prevent_primary_email_deletion();

-- Remove the trigger that prevents the first email without primary flag
DROP TRIGGER trigger_prevent_first_email_without_primary ON emails;
DROP FUNCTION prevent_first_email_without_primary();

-- Remove the primary column from emails table
ALTER TABLE emails DROP COLUMN is_primary;

-- Remove the GiST extension if it is no longer needed
DROP EXTENSION IF EXISTS btree_gist;

-- Retain just the first email for each user
DELETE FROM emails
WHERE user_id IN (SELECT user_id FROM emails GROUP BY user_id HAVING COUNT(*) > 1)
AND id NOT IN (
  SELECT MIN(id) FROM emails GROUP BY user_id
);

-- Re-add the unique constraint on user_id to enforce single email per user
ALTER TABLE emails ADD CONSTRAINT emails_user_id_key UNIQUE (user_id);

