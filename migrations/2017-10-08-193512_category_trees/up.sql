-- Your SQL goes here
CREATE EXTENSION ltree;

-- Create the new column which will represent our category tree.
-- Fill it with values from `slug` column and then set to non-null
ALTER TABLE categories
    ADD COLUMN path ltree;

-- Unfortunately, hyphens (dashes) are not allowed...
UPDATE categories
    SET path =  text2ltree('root.' || trim(replace(replace(slug, '-', '_'), '::', '.')))
    WHERE path is NULL;

ALTER TABLE CATEGORIES
    ALTER COLUMN path SET NOT NULL;

-- Create some indices that allow us to use GIST operators: '@>', etc
CREATE INDEX path_gist_categories_idx ON categories USING GIST(path);
CREATE INDEX path_categories_idx ON categories USING btree(path);

-- Create procedure and trigger to auto-update path
CREATE OR REPLACE FUNCTION set_category_path_to_slug()
  RETURNS trigger AS
    $BODY$
BEGIN
 NEW.path = text2ltree('root.' || trim(replace(replace(NEW.slug, '-', '_'), '::', '.')));
 RETURN NEW;
END;
    $BODY$ LANGUAGE plpgsql;

CREATE TRIGGER set_category_path_insert
  BEFORE INSERT
  ON categories
  FOR EACH ROW
  EXECUTE PROCEDURE set_category_path_to_slug();

CREATE TRIGGER set_category_path_update
  BEFORE UPDATE OF slug ON categories
  FOR EACH ROW
  EXECUTE PROCEDURE set_category_path_to_slug();
