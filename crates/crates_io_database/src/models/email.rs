use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use secrecy::SecretString;

use crate::models::User;
use crate::schema::emails;

#[derive(Debug, Queryable, Identifiable, Associations)]
#[diesel(belongs_to(User))]
pub struct Email {
    pub id: i32,
    pub user_id: i32,
    pub email: String,
    pub verified: bool,
    #[diesel(deserialize_as = String, serialize_as = String)]
    pub token: SecretString,
    pub token_generated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable, AsChangeset, Builder)]
#[diesel(table_name = emails, check_for_backend(diesel::pg::Pg))]
pub struct NewEmail<'a> {
    pub user_id: i32,
    pub email: &'a str,
    #[builder(default = false)]
    pub verified: bool,
}

impl NewEmail<'_> {
    pub async fn insert(&self, conn: &mut AsyncPgConnection) -> QueryResult<()> {
        diesel::insert_into(emails::table)
            .values(self)
            .execute(conn)
            .await?;

        Ok(())
    }

    /// Inserts the email into the database and returns the confirmation token,
    /// or does nothing if it already exists and returns `None`.
    pub async fn insert_if_missing(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<Option<SecretString>> {
        diesel::insert_into(emails::table)
            .values(self)
            .on_conflict_do_nothing()
            .returning(emails::token)
            .get_result::<String>(conn)
            .await
            .map(Into::into)
            .optional()
    }

    pub async fn insert_or_update(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<SecretString> {
        diesel::insert_into(emails::table)
            .values(self)
            .on_conflict(emails::user_id)
            .do_update()
            .set(self)
            .returning(emails::token)
            .get_result::<String>(conn)
            .await
            .map(Into::into)
    }
}
