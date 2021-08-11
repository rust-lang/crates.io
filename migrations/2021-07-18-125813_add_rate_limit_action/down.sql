DELETE FROM publish_limit_buckets WHERE action != 0;
ALTER TABLE publish_limit_buckets DROP CONSTRAINT publish_limit_buckets_pkey;
ALTER TABLE publish_limit_buckets ADD CONSTRAINT publish_limit_buckets_pkey PRIMARY KEY (user_id);
ALTER TABLE publish_limit_buckets DROP COLUMN action;

DELETE FROM publish_rate_overrides WHERE action != 0;
ALTER TABLE publish_rate_overrides DROP CONSTRAINT publish_rate_overrides_pkey;
ALTER TABLE publish_rate_overrides ADD CONSTRAINT publish_rate_overrides_pkey PRIMARY KEY (user_id);
ALTER TABLE publish_rate_overrides DROP COLUMN action;
