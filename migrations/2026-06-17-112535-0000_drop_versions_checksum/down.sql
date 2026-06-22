-- This only restores the column shape; the dropped checksum data is gone.
ALTER TABLE versions ADD COLUMN checksum char(64);
