CREATE TABLE crate_owner_invitations (
    id SERIAL PRIMARY KEY,
    invited_user_id INTEGER NOT NULL REFERENCES users (id),
    invited_by INTEGER NOT NULL REFERENCES users (id),
    crate_id INTEGER NOT NULL REFERENCES crates (id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    created TIMESTAMP NOT NULL DEFAULT now()
);

CREATE FUNCTION ensure_single_crate_owner_invitation() RETURNS trigger AS $$
DECLARE
    old_id INTEGER;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        old_id = OLD.id;
    ELSE
        old_id = -1;
    END IF;

    -- If a pending invitation already exists for this same invited user and crate
    IF EXISTS (
        SELECT 1 FROM crate_owner_invitations
            WHERE id != old_id AND
                  invited_user_id = NEW.invited_user_id AND
                  crate_id = NEW.crate_id AND
                  status = 'pending'
    ) THEN
        RAISE EXCEPTION 'cannot invite a user to the same crate more than once';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_ensure_single_crate_owner_invitation
BEFORE INSERT OR UPDATE ON crate_owner_invitations
FOR EACH ROW EXECUTE PROCEDURE ensure_single_crate_owner_invitation();
