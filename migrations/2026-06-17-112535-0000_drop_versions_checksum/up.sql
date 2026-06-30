-- safety-assured:start
-- The previous release stopped reading and writing `checksum` entirely, so no
-- running code references the column when it is dropped.
ALTER TABLE versions DROP COLUMN checksum;
-- safety-assured:end
