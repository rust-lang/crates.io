ALTER TABLE crate_owner_invitations ADD COLUMN token TEXT NOT NULL DEFAULT random_string(26);
ALTER TABLE crate_owner_invitations ADD COLUMN token_generated_at TIMESTAMP;

CREATE FUNCTION crate_owner_invitations_set_token_generated_at() RETURNS trigger AS $$
  BEGIN
    NEW.token_generated_at := CURRENT_TIMESTAMP;
    RETURN NEW;
  END
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_crate_owner_invitations_set_token_generated_at BEFORE
INSERT OR UPDATE OF token ON crate_owner_invitations
FOR EACH ROW EXECUTE PROCEDURE crate_owner_invitations_set_token_generated_at();
