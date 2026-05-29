-- safety-assured:start
-- Dropping the `users` view and renaming `users_v2` back to `users` look
-- dangerous, but no application code references `users_v2` (the table was
-- only ever named that to make room for the compatibility view), and the
-- rename happens in the same transaction as the view drop so concurrent
-- queries against `users` see either the old view or the renamed table.
DROP VIEW users;
ALTER TABLE users_v2 RENAME TO users;
-- safety-assured:end
