ALTER TABLE crate_owners DROP CONSTRAINT crate_owners_pkey;
ALTER TABLE crate_owners ADD CONSTRAINT crate_owners_unique_owner_per_crate UNIQUE (crate_id, owner_id, owner_kind);
ALTER TABLE crate_owners ADD COLUMN id SERIAL PRIMARY KEY;
CREATE INDEX index_crate_owners_crate_id ON crate_owners (crate_id);
DROP INDEX crate_owners_not_deleted;
