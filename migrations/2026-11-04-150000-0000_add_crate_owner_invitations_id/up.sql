-- safety-assured:start
-- Filling in the identity column rewrites the table, and swapping the primary
-- key builds two indexes, all while holding an `ACCESS EXCLUSIVE` lock. The
-- table holds about 5k rows, so the lock is short.
--
-- The previous code keeps working during the deploy. It refers to invitations
-- by `(invited_user_id, crate_id)` in its queries, which stays unique, and its
-- `ON CONFLICT DO NOTHING` insert relies on that uniqueness.
ALTER TABLE crate_owner_invitations
ADD COLUMN IF NOT EXISTS id BIGINT GENERATED ALWAYS AS IDENTITY;

ALTER TABLE crate_owner_invitations
DROP CONSTRAINT crate_owner_invitations_pkey;

ALTER TABLE crate_owner_invitations
ADD CONSTRAINT crate_owner_invitations_pkey PRIMARY KEY (id);

ALTER TABLE crate_owner_invitations
ADD CONSTRAINT crate_owner_invitations_invited_user_id_crate_id_key UNIQUE (invited_user_id, crate_id);
-- safety-assured:end

COMMENT ON COLUMN crate_owner_invitations.id IS 'Unique identifier of the invitation, used to refer to it in the API.';
