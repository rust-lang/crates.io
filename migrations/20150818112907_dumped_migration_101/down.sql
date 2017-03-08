ALTER TABLE crate_owners DROP CONSTRAINT crate_owners_unique_owner_per_crate;
ALTER TABLE crate_owners ADD CONSTRAINT crate_owners_unique_user_per_crate UNIQUE (owner_id, crate_id);