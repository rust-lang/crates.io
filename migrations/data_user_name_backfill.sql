-- Backfill `users.name` from `oauth_github.login` for users whose `name` is
-- still NULL. `users.name` is currently set only when the GitHub profile has
-- a display name, so users without one have NULL there; this fills those in
-- with the GitHub username so every account has a non-NULL name.
--
-- Iterates by `users.id` range and COMMITs between batches so row locks
-- release between iterations and concurrent traffic isn't blocked. The DO
-- block must be run outside an explicit transaction (`psql -f <file>` is
-- fine; do NOT wrap with BEGIN/COMMIT, since COMMIT inside DO requires the
-- block to be at the top level).
--
-- Idempotent: re-running the file is a no-op once every account has a name,
-- because the UPDATE filters on `users.name IS NULL`.

SET lock_timeout = '5s';
SET statement_timeout = '60s';

DO $$
DECLARE
    lo INT;
    hi INT;
    pos INT;
    batch_size CONSTANT INT := 5000;
BEGIN
    SELECT MIN(id), MAX(id) INTO lo, hi FROM users WHERE name IS NULL;
    IF lo IS NULL THEN RETURN; END IF;

    pos := lo;
    WHILE pos <= hi LOOP
        UPDATE users
        SET name = oauth_github.login
        FROM oauth_github
        WHERE oauth_github.user_id = users.id
          AND users.name IS NULL
          AND users.id >= pos
          AND users.id < pos + batch_size;
        COMMIT;
        pos := pos + batch_size;
    END LOOP;
END $$;
