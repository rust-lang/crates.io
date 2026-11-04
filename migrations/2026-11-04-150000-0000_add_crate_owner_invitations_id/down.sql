ALTER TABLE crate_owner_invitations
DROP CONSTRAINT IF EXISTS crate_owner_invitations_invited_user_id_crate_id_key;

ALTER TABLE crate_owner_invitations
DROP CONSTRAINT IF EXISTS crate_owner_invitations_pkey;

ALTER TABLE crate_owner_invitations
ADD CONSTRAINT crate_owner_invitations_pkey PRIMARY KEY (invited_user_id, crate_id);

ALTER TABLE crate_owner_invitations
DROP COLUMN IF EXISTS id;
