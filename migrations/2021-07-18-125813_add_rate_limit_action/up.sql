ALTER TABLE publish_limit_buckets ADD COLUMN action INTEGER NOT NULL DEFAULT 0;
ALTER TABLE publish_limit_buckets DROP CONSTRAINT publish_limit_buckets_pkey;
ALTER TABLE publish_limit_buckets ADD CONSTRAINT publish_limit_buckets_pkey PRIMARY KEY (user_id, action);

ALTER TABLE publish_rate_overrides ADD COLUMN action INTEGER NOT NULL DEFAULT 0;
ALTER TABLE publish_rate_overrides DROP CONSTRAINT publish_rate_overrides_pkey;
ALTER TABLE publish_rate_overrides ADD CONSTRAINT publish_rate_overrides_pkey PRIMARY KEY (user_id, action);
