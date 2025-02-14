-- Delete crate owners that are associated with teams that have no `org_id`
with broken_crate_owners as (
    select crate_id, owner_id, owner_kind
    from crate_owners
      left join teams on teams.id = crate_owners.owner_id
    where owner_kind = 1
      and teams.org_id is null
)
delete from crate_owners
  using broken_crate_owners
where crate_owners.crate_id = broken_crate_owners.crate_id
  and crate_owners.owner_id = broken_crate_owners.owner_id
  and crate_owners.owner_kind = broken_crate_owners.owner_kind;

-- Delete teams that have no `org_id`
delete from teams where org_id is null;

-- Make `org_id` not null
alter table teams alter column org_id set not null;
