-- Remove trustpub_data column from versions table
ALTER TABLE versions DROP COLUMN trustpub_data;

-- Remove trustpub_data column from trustpub_tokens table  
ALTER TABLE trustpub_tokens DROP COLUMN trustpub_data;
