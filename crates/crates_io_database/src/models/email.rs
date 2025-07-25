use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::upsert::on_constraint;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use secrecy::SecretString;

use crate::models::User;
use crate::schema::emails;

#[derive(Debug, Queryable, Identifiable, Selectable, Associations)]
#[diesel(belongs_to(User))]
pub struct Email {
    pub id: i32,
    pub user_id: i32,
    pub email: String,
    pub verified: bool,
    pub primary: bool,
    #[diesel(deserialize_as = String, serialize_as = String)]
    pub token: SecretString,
    pub token_generated_at: Option<DateTime<Utc>>,
}

impl Email {
    pub async fn find(conn: &mut AsyncPgConnection, id: i32) -> QueryResult<Self> {
        emails::table
            .find(id)
            .select(Email::as_select())
            .first(conn)
            .await
    }
}

#[derive(Debug, Insertable, AsChangeset, Builder)]
#[diesel(table_name = emails, check_for_backend(diesel::pg::Pg))]
pub struct NewEmail<'a> {
    pub user_id: i32,
    pub email: &'a str,
    #[builder(default = false)]
    pub verified: bool,
    #[builder(default = false)]
    pub primary: bool,
}

impl NewEmail<'_> {
    pub async fn insert(&self, conn: &mut AsyncPgConnection) -> QueryResult<Email> {
        diesel::insert_into(emails::table)
            .values(self)
            .returning(Email::as_returning())
            .get_result(conn)
            .await
    }

    /// Inserts the email into the database and returns the email record,
    /// or does nothing if it already exists and returns `None`.
    pub async fn insert_if_missing(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<Option<Email>> {
        diesel::insert_into(emails::table)
            .values(self)
            .on_conflict(on_constraint("unique_user_email"))
            .do_nothing()
            .returning(Email::as_returning())
            .get_result(conn)
            .await
            .optional()
    }
}
