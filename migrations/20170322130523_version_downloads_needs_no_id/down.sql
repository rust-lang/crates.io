ALTER TABLE version_downloads ALTER COLUMN "date" TYPE timestamp;
DROP INDEX version_downloads_unique;
