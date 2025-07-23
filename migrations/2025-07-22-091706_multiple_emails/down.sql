-- Remove the function for enabling notifications for an email
DROP FUNCTION enable_notifications_for_email;

-- Remove the function that enforces the maximum number of emails per user
DROP TRIGGER trigger_enforce_max_emails_per_user ON emails;
DROP FUNCTION enforce_max_emails_per_user();

-- Remove the unique constraint for the combination of user_id and email
ALTER TABLE emails DROP CONSTRAINT unique_user_email;

-- Remove the constraint that allows only one notification email per user
ALTER TABLE emails DROP CONSTRAINT unique_notification_email_per_user;

-- Remove the trigger that enforces at least one notification email per user
DROP TRIGGER trigger_ensure_at_least_one_notification_email ON emails;
DROP FUNCTION ensure_at_least_one_notification_email();

-- Remove the trigger that prevents deletion of emails with notifications enabled
DROP TRIGGER trigger_prevent_notification_email_deletion ON emails;
DROP FUNCTION prevent_notification_email_deletion();

-- Remove the trigger that prevents the first email without notifications
DROP TRIGGER trigger_prevent_first_email_without_notifications ON emails;
DROP FUNCTION prevent_first_email_without_notifications();

-- Remove the send_notifications column from emails table
ALTER TABLE emails DROP COLUMN send_notifications;

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

