-- This file should undo anything in `up.sql`
ALTER TABLE crates ADD COLUMN license VARCHAR;
