-- Diesel always runs migrations in a transaction, and we can't create indexes
-- concurrently in a transaction. We can't block crate downloads on this, so
-- just raise if we try to run this in production
DO $$
DECLARE
  tmp_index_exists boolean;
BEGIN
  tmp_index_exists := (SELECT EXISTS (
      SELECT * FROM pg_index
        INNER JOIN pg_class
          ON pg_class.oid = pg_index.indexrelid
        WHERE relname = 'version_downloads_tmp'
  ));
  IF NOT tmp_index_exists THEN
    IF (SELECT COUNT(*) FROM version_downloads) > 1000 THEN
      RAISE EXCEPTION 'Indexes need to be created concurrently in production, manually create them and try again';
    ELSE
      ALTER TABLE version_downloads ADD COLUMN id SERIAL;
      CREATE UNIQUE INDEX version_downloads_tmp ON version_downloads (id);
      CREATE UNIQUE INDEX version_downloads_unique ON version_downloads (version_id, date);
      CREATE UNIQUE INDEX index_version_downloads_not_processed ON version_downloads (id) WHERE processed = false;
    END IF;
  END IF;
END $$;

ALTER TABLE version_downloads
  DROP CONSTRAINT version_downloads_pkey,
  ADD CONSTRAINT version_downloads_pkey PRIMARY KEY USING INDEX version_downloads_tmp;
