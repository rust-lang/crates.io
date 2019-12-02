ALTER TABLE version_owner_actions
ALTER COLUMN user_id
DROP NOT NULL
;

ALTER TABLE version_owner_actions
ALTER COLUMN version_id
DROP NOT NULL
;

ALTER TABLE version_owner_actions
RENAME COLUMN user_id TO owner_id
;

ALTER TABLE version_owner_actions
RENAME COLUMN api_token_id TO owner_token_id
;
