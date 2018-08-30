ALTER TABLE versions ALTER COLUMN features SET DATA TYPE jsonb USING features::jsonb;
ALTER TABLE versions ALTER COLUMN features SET DEFAULT '{}';
ALTER TABLE versions ALTER COLUMN features SET NOT NULL;
