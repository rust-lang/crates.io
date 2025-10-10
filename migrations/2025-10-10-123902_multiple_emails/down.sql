-- Remove the function for promoting an email to primary
DROP FUNCTION promote_email_to_primary;

-- Remove the function that enforces the maximum number of emails per user
DROP TRIGGER trigger_enforce_max_emails_per_user ON emails;
DROP FUNCTION enforce_max_emails_per_user();

-- Remove the unique constraint for the combination of user_id and email
ALTER TABLE emails DROP CONSTRAINT unique_user_email;

-- Remove the constraint that allows only one primary email per user
ALTER TABLE emails DROP CONSTRAINT unique_primary_email_per_user;

-- Remove the trigger that prevents deletion of primary emails
DROP TRIGGER trigger_prevent_primary_email_deletion ON emails;
DROP FUNCTION prevent_primary_email_deletion();

-- Remove the trigger that ensures exactly one primary email per user
DROP TRIGGER trigger_verify_exactly_one_primary_email ON emails;
DROP FUNCTION verify_exactly_one_primary_email();

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
