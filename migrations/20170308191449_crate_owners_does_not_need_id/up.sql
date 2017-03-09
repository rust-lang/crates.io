ALTER TABLE crate_owners DROP COLUMN id;
ALTER TABLE crate_owners DROP CONSTRAINT crate_owners_unique_owner_per_crate;
ALTER TABLE crate_owners ADD PRIMARY KEY (crate_id, owner_id, owner_kind);
DROP INDEX index_crate_owners_crate_id;
CREATE UNIQUE INDEX crate_owners_not_deleted ON crate_owners (crate_id, owner_id, owner_kind) WHERE NOT deleted;
