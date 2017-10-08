-- This file should undo anything in `up.sql`
DROP TRIGGER set_category_path_update ON categories;

DROP TRIGGER set_category_path_insert ON categories;

DROP FUNCTION IF EXISTS set_category_path_to_slug();

DROP INDEX path_categories_idx;
DROP INDEX path_gist_categories_idx;

ALTER TABLE categories
    DROP COLUMN path;

DROP EXTENSION ltree;
