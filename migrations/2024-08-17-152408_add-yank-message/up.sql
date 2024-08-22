ALTER TABLE versions ADD COLUMN yank_message TEXT;

COMMENT ON COLUMN versions.yank_message IS 'message associated with a yanked version';
