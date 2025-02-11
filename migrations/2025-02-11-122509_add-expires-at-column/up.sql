alter table crate_owner_invitations add column expires_at timestamptz;

comment on column public.crate_owner_invitations.expires_at is 'Point in time at which the invitation expires/expired.'

-- to be performed manually after the migration:
--
-- update table crate_owner_invitations set expires_at = now() + interval '30 day';
