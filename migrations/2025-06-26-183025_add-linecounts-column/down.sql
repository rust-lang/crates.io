-- Remove line count statistics column from versions table
ALTER TABLE versions 
DROP COLUMN linecounts;