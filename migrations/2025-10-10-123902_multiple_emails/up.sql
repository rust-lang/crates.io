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

-- Limit primary flag to one email per user
-- Evaluation of the constraint is deferred to the end of the transaction to allow for replacement of the primary email
CREATE EXTENSION IF NOT EXISTS btree_gist;
ALTER TABLE emails ADD CONSTRAINT unique_primary_email_per_user
EXCLUDE USING gist (
  user_id WITH =,
  (is_primary::int) WITH =
)
WHERE (is_primary)
DEFERRABLE INITIALLY DEFERRED;

-- Prevent deletion of primary email, unless it's the only email for that user
CREATE FUNCTION prevent_primary_email_deletion()
RETURNS TRIGGER AS $$
BEGIN
  IF OLD.is_primary IS TRUE THEN
    -- Allow deletion if this is the only email for the user
    IF (SELECT COUNT(*) FROM emails WHERE user_id = OLD.user_id) = 1 THEN
      RETURN OLD;
    END IF;
    RAISE EXCEPTION 'Cannot delete primary email. Please set another email as primary first.';
  END IF;
  RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_prevent_primary_email_deletion
BEFORE DELETE ON emails
FOR EACH ROW
EXECUTE FUNCTION prevent_primary_email_deletion();

-- Ensure exactly one primary email per user after any insert or update
CREATE FUNCTION verify_exactly_one_primary_email()
RETURNS TRIGGER AS $$
DECLARE
  primary_count integer;
BEGIN
  -- Count primary emails for the affected user
  SELECT COUNT(*) INTO primary_count
  FROM emails
  WHERE user_id = COALESCE(NEW.user_id, OLD.user_id)
  AND is_primary = true;

  IF primary_count != 1 THEN
    RAISE EXCEPTION 'User must have one primary email, found %', primary_count;
  END IF;

  RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_verify_exactly_one_primary_email
AFTER INSERT OR UPDATE ON emails
FOR EACH ROW
EXECUTE FUNCTION verify_exactly_one_primary_email();

-- Function to set the primary flag to true for an existing email
-- This will set the flag to false for all other emails of the same user
CREATE FUNCTION promote_email_to_primary(target_email_id integer)
RETURNS void AS $$
DECLARE
  target_user_id integer;
BEGIN
  SELECT user_id INTO target_user_id FROM emails WHERE id = target_email_id;
  IF target_user_id IS NULL THEN
    RAISE EXCEPTION 'Email ID % does not exist', target_email_id;
  END IF;

  UPDATE emails
  SET is_primary = (id = target_email_id)
  WHERE user_id = target_user_id;
END;
$$ LANGUAGE plpgsql;
