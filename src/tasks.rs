pub mod dump_db;
mod update_downloads;

pub use dump_db::dump_db;
pub use update_downloads::update_downloads;

use diesel::sql_types::BigInt;
sql_function!(fn pg_try_advisory_xact_lock(key: BigInt) -> Bool);

const UPDATE_DOWNLOADS_ADVISORY_LOCK_KEY: i64 = 1;
