SET timezone = 'UTC';

alter table api_tokens alter created_at type timestamptz;
alter table api_tokens alter last_used_at type timestamptz;
alter table api_tokens alter expired_at type timestamptz;
alter table api_tokens alter expiry_notification_at type timestamptz;

alter table background_jobs alter last_retry type timestamptz;
alter table background_jobs alter created_at type timestamptz;

alter table categories alter created_at type timestamptz;

alter table crate_owner_invitations alter created_at type timestamptz;

alter table crate_owners alter created_at type timestamptz;
alter table crate_owners alter updated_at type timestamptz;

drop trigger trigger_crates_tsvector_update on crates;

alter table crates alter created_at type timestamptz;
alter table crates alter updated_at type timestamptz;

create trigger trigger_crates_tsvector_update
    before insert or update
        of updated_at
    on crates
    for each row
execute procedure trigger_crates_name_search();

alter table emails alter token_generated_at type timestamptz;

alter table keywords alter created_at type timestamptz;

alter table publish_limit_buckets alter last_refill type timestamptz;

alter table publish_rate_overrides alter expires_at type timestamptz;

alter table readme_renderings alter rendered_at type timestamptz;

alter table users alter account_lock_until type timestamptz;

alter table version_owner_actions alter time type timestamptz;

alter table versions alter updated_at type timestamptz;
alter table versions alter created_at type timestamptz;
