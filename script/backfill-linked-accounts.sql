INSERT INTO linked_accounts (user_id, provider, account_id, access_token, login, avatar)
SELECT id, 0, gh_id, gh_access_token, gh_login, gh_avatar
FROM users
LEFT JOIN linked_accounts
ON users.id = linked_accounts.user_id
WHERE gh_id != -1
AND linked_accounts.user_id IS NULL;
