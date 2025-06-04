use crate::schema::versions_published_by;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

pub async fn insert(
    version_id: i32,
    email: &str,
    conn: &mut AsyncPgConnection,
) -> QueryResult<usize> {
    diesel::insert_into(versions_published_by::table)
        .values((
            versions_published_by::version_id.eq(version_id),
            versions_published_by::email.eq(email),
        ))
        .execute(conn)
        .await
}
