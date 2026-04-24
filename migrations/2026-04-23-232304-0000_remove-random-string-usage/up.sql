ALTER TABLE crate_owner_invitations
ALTER COLUMN token
DROP DEFAULT;

ALTER TABLE emails
ALTER COLUMN token
DROP DEFAULT;

DROP FUNCTION random_string;
