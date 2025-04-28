use crate::schema::trustpub_tokens;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

#[derive(Debug, Insertable)]
#[diesel(table_name = trustpub_tokens, check_for_backend(diesel::pg::Pg))]
pub struct NewToken<'a> {
    pub expires_at: DateTime<Utc>,
    pub hashed_token: &'a [u8],
    pub crate_ids: &'a [i32],
}

impl NewToken<'_> {
    pub async fn insert(&self, conn: &mut AsyncPgConnection) -> QueryResult<()> {
        self.insert_into(trustpub_tokens::table)
            .execute(conn)
            .await?;

        Ok(())
    }
}
