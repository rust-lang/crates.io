use crate::schema::trustpub_used_jtis;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

#[derive(Debug, Insertable)]
#[diesel(table_name = trustpub_used_jtis, check_for_backend(diesel::pg::Pg))]
pub struct NewUsedJti<'a> {
    pub jti: &'a str,
    pub expires_at: DateTime<Utc>,
}

impl<'a> NewUsedJti<'a> {
    pub fn new(jti: &'a str, expires_at: DateTime<Utc>) -> Self {
        Self { jti, expires_at }
    }

    pub async fn insert(&self, conn: &mut AsyncPgConnection) -> QueryResult<usize> {
        diesel::insert_into(trustpub_used_jtis::table)
            .values(self)
            .execute(conn)
            .await
    }
}
