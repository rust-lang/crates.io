-- Drop the unique constraint on user_id to allow multiple emails per user
ALTER TABLE emails DROP CONSTRAINT emails_user_id_key;

-- Limit users to 32 emails maximum
CREATE FUNCTION enforce_max_emails_per_user()
RETURNS TRIGGER AS $$
BEGIN
  IF (SELECT COUNT(*) FROM emails WHERE user_id = NEW.user_id) > 32 THEN
    RAISE EXCEPTION 'User cannot have more than 32 emails';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_enforce_max_emails_per_user
BEFORE INSERT ON emails
FOR EACH ROW
EXECUTE FUNCTION enforce_max_emails_per_user();

-- Add a unique constraint for the combination of user_id and email
ALTER TABLE emails ADD CONSTRAINT unique_user_email UNIQUE (user_id, email);

-- Add a new column for identifying if an email should receive notifications
ALTER TABLE emails ADD COLUMN send_notifications BOOLEAN DEFAULT FALSE NOT NULL;

-- Set `send_notifications` to true for existing emails
UPDATE emails SET send_notifications = true;

-- Limit notification flag to one email per user
-- Evaluation of the constraint is deferred to the end of the transaction to allow for replacement of the notification email
CREATE EXTENSION IF NOT EXISTS btree_gist;
ALTER TABLE emails ADD CONSTRAINT unique_notification_email_per_user
EXCLUDE USING gist (
  user_id WITH =,
  (send_notifications::int) WITH =
)
WHERE (send_notifications)
DEFERRABLE INITIALLY DEFERRED;

-- Prevent deletion of emails if they have notifications enabled, unless it's the only email for that user
CREATE FUNCTION prevent_notification_email_deletion()
RETURNS TRIGGER AS $$
BEGIN
  IF OLD.send_notifications IS TRUE THEN
    -- Allow deletion if this is the only email for the user
    IF (SELECT COUNT(*) FROM emails WHERE user_id = OLD.user_id) = 1 THEN
      RETURN OLD;
    END IF;
    RAISE EXCEPTION 'Cannot delete email: send_notifications is set to true';
  END IF;
  RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_prevent_notification_email_deletion
BEFORE DELETE ON emails
FOR EACH ROW
EXECUTE FUNCTION prevent_notification_email_deletion();

-- Prevent creation of first email for a user if notifications are disabled
CREATE FUNCTION prevent_first_email_without_notifications()
RETURNS TRIGGER AS $$
BEGIN
  -- Count the current emails for this user_id
  IF NOT EXISTS (
    SELECT 1 FROM emails WHERE user_id = NEW.user_id
  ) AND NEW.send_notifications IS NOT TRUE THEN
    RAISE EXCEPTION 'The first email for a user must have send_notifications = true';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_prevent_first_email_without_notifications
BEFORE INSERT ON emails
FOR EACH ROW
EXECUTE FUNCTION prevent_first_email_without_notifications();

-- Ensure that at least one email for the user has send_notifications = true, unless the user has no emails
-- Using a trigger-based approach since exclusion constraints cannot use subqueries
CREATE FUNCTION ensure_at_least_one_notification_email()
RETURNS TRIGGER AS $$
BEGIN
  -- Check if this operation would leave the user without any notification emails
  IF (TG_OP = 'UPDATE' AND OLD.send_notifications = true AND NEW.send_notifications = false) OR
     (TG_OP = 'DELETE' AND OLD.send_notifications = true) THEN
    -- Skip check if user has no emails left
    IF NOT EXISTS (SELECT 1 FROM emails WHERE user_id = OLD.user_id AND id != OLD.id) THEN
      RETURN COALESCE(NEW, OLD);
    END IF;

    IF NOT EXISTS (
      SELECT 1 FROM emails
      WHERE user_id = OLD.user_id
      AND send_notifications = true
      AND id != OLD.id
    ) THEN
      RAISE EXCEPTION 'Each user must have at least one email with send_notifications = true';
    END IF;
  END IF;

  RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_ensure_at_least_one_notification_email
AFTER UPDATE OR DELETE ON emails
FOR EACH ROW
EXECUTE FUNCTION ensure_at_least_one_notification_email();

-- Function to set the send_notifications flag to true for an existing email
-- This will set the flag to false for all other emails of the same user
CREATE FUNCTION enable_notifications_for_email(target_email_id integer)
RETURNS void AS $$
DECLARE
  target_user_id integer;
BEGIN
  SELECT user_id INTO target_user_id FROM emails WHERE id = target_email_id;
  IF target_user_id IS NULL THEN
    RAISE EXCEPTION 'Email ID % does not exist', target_email_id;
  END IF;

  UPDATE emails
  SET send_notifications = (id = target_email_id)
  WHERE user_id = target_user_id;
END;
$$ LANGUAGE plpgsql;
