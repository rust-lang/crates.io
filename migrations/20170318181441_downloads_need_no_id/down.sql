ALTER TABLE crate_downloads DROP CONSTRAINT crate_downloads_pkey
ALTER TABLE crate_downloads ALTER COLUMN "date" TYPE timestamp;
ALTER TABLE crate_downloads ADD COLUMN id SERIAL PRIMARY KEY;
