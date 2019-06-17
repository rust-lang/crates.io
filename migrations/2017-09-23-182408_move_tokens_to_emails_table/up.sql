ALTER TABLE emails ADD COLUMN token TEXT NOT NULL DEFAULT random_string(26);
ALTER TABLE emails ADD COLUMN token_generated_at TIMESTAMP;
UPDATE emails SET token = tokens.token, token_generated_at = tokens.created_at
  FROM tokens WHERE tokens.email_id = emails.id;
DROP TABLE tokens;

CREATE FUNCTION emails_set_token_generated_at() RETURNS trigger AS $$
  BEGIN
    NEW.token_generated_at := CURRENT_TIMESTAMP;
    RETURN NEW;
  END
$$ LANGUAGE plpgsql;

CREATE FUNCTION reconfirm_email_on_email_change() RETURNS trigger AS $$
  BEGIN
    IF NEW.email IS DISTINCT FROM OLD.email THEN
      NEW.token := random_string(26);
      NEW.verified := false;
    END IF;
    RETURN NEW;
  END
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_emails_set_token_generated_at BEFORE
INSERT OR UPDATE OF token ON emails
FOR EACH ROW EXECUTE PROCEDURE emails_set_token_generated_at();

CREATE TRIGGER trigger_emails_reconfirm BEFORE UPDATE
ON emails
FOR EACH ROW EXECUTE PROCEDURE reconfirm_email_on_email_change();
