mod infer_users {
    infer_table_from_schema!("dotenv:DATABASE_URL", "users");
}
pub use self::infer_users::*;
