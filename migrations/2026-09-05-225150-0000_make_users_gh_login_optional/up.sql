-- safety-assured:start
-- This column is not being read anywhere anymore. All reads are now using
-- `oauth_github.login` (an associated GitHub account is optional and all code paths have
-- been updated to reflect that). This change means we can stop writing this column but not yet
-- drop it.
ALTER TABLE users ALTER COLUMN gh_login DROP NOT NULL;
-- safety-assured:end
