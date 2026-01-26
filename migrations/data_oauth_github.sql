-- Duplicate GitHub-related user info from the `users` table to the `oauth_github` table. The
-- `users` table is still the source of truth, but the code should be updating this table as well.
--
-- This migration is safe to run multiple times.
INSERT INTO oauth_github
(user_id, account_id, encrypted_token, login, avatar)
SELECT
  id as user_id,
  gh_id as account_id,
  gh_encrypted_token as encrypted_token,
  gh_login as login,
  gh_avatar as avatar
FROM users
WHERE users.gh_id > 0
ON CONFLICT DO NOTHING;
