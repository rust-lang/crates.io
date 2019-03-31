-- I expect that it will be save to apply this migration in production as the code that adds records
-- to this table is not used until this changeset.

ALTER TABLE version_owner_actions
RENAME COLUMN owner_id TO user_id
;

ALTER TABLE version_owner_actions
RENAME COLUMN owner_token_id TO api_token_id
;

ALTER TABLE version_owner_actions
ALTER COLUMN user_id
SET NOT NULL
;

ALTER TABLE version_owner_actions
ALTER COLUMN version_id
SET NOT NULL
;
