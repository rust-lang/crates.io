use std::error::Error;

use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel::PgConnection;

sql_function!(fn pg_try_advisory_lock(key: BigInt) -> Bool);
sql_function!(fn pg_advisory_unlock(key: BigInt) -> Bool);

/// Run the callback if the session advisory lock for the given key can be obtained.
///
/// If the lock is not already held, the callback will be called and the lock will be unlocked
/// after the closure returns.  If the lock is already held, the function returns an error without
/// calling the callback.
pub(crate) fn with_advisory_lock<F>(
    conn: &PgConnection,
    key: i64,
    f: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(&PgConnection) -> QueryResult<()>,
{
    if !diesel::select(pg_try_advisory_lock(key)).get_result(conn)? {
        let string = format!(
            "A job holding the session advisory lock for key {} is already running",
            key
        );
        println!("return");
        return Err(string.into());
    }
    println!("umm");
    let _dont_drop_yet = DropGuard { conn, key };
    f(conn).map_err(Into::into)
}

struct DropGuard<'a> {
    conn: &'a PgConnection,
    key: i64,
}

impl<'a> Drop for DropGuard<'a> {
    fn drop(&mut self) {
        match diesel::select(pg_advisory_unlock(self.key)).get_result(self.conn) {
            Ok(true) => (),
            Ok(false) => println!(
                "Error: job advisory lock for key {} was not locked",
                self.key
            ),
            Err(err) => println!("Error unlocking advisory lock (key: {}): {}", self.key, err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn lock_released_after_callback_returns() {
        const KEY: i64 = -1;
        let conn1 = pg_connection();
        let conn2 = pg_connection();

        let mut callback_run = false;
        let result = with_advisory_lock(&conn1, KEY, |_| {
            // Another connection cannot obtain the lock
            assert!(!diesel::select(pg_try_advisory_lock(KEY)).get_result(&conn2)?);
            callback_run = true;
            Ok(())
        });
        assert!(result.is_ok());
        assert!(callback_run);

        // Another connection can now obtain the lock
        assert_eq!(
            diesel::select(pg_try_advisory_lock(KEY)).get_result(&conn2),
            Ok(true)
        );
    }

    #[test]
    fn test_already_locked() {
        const KEY: i64 = -2;
        let conn1 = pg_connection();
        let conn2 = pg_connection();

        // Another connection obtains the lock first
        assert_eq!(
            diesel::select(pg_try_advisory_lock(KEY)).get_result(&conn2),
            Ok(true)
        );

        let mut callback_run = false;
        let result = with_advisory_lock(&conn1, KEY, |_| {
            callback_run = true;
            Ok(())
        });
        assert!(dbg!(result).is_err());
        assert!(!callback_run);
    }
}
