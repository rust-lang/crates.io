use bon::Builder;
use diesel::prelude::*;
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
    pub is_primary: bool,
    #[diesel(deserialize_as = String, serialize_as = String)]
    pub token: SecretString,
}

#[derive(Debug, Insertable, AsChangeset, Builder)]
#[diesel(table_name = emails, check_for_backend(diesel::pg::Pg))]
pub struct NewEmail<'a> {
    pub user_id: i32,
    pub email: &'a str,
    #[builder(default = false)]
    pub verified: bool,
    #[builder(default = false)]
    pub is_primary: bool,
}

impl NewEmail<'_> {
    pub async fn insert(&self, conn: &mut AsyncPgConnection) -> QueryResult<Email> {
        diesel::insert_into(emails::table)
            .values(self)
            .returning(Email::as_returning())
            .get_result(conn)
            .await
    }

    /// Inserts the email into the database and returns it, unless the user already has a
    /// primary email, in which case it will do nothing and return `None`.
    pub async fn insert_primary_if_missing(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<Option<Email>> {
        // Check if the user already has a primary email
        let primary_count = emails::table
            .filter(emails::user_id.eq(self.user_id))
            .filter(emails::is_primary.eq(true))
            .count()
            .get_result::<i64>(conn)
            .await?;

        if primary_count > 0 {
            return Ok(None); // User already has a primary email
        }

        self.insert(conn).await.map(Some)
    }

    // Inserts an email for the user, replacing the primary email if it exists.
    pub async fn insert_or_update_primary(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<Email> {
        if self.is_primary {
            return Err(diesel::result::Error::QueryBuilderError(
                "Cannot use insert_or_update_primary with a non-primary email".into(),
            ));
        }

        // Attempt to update an existing primary email
        let updated_email = diesel::update(
            emails::table
                .filter(emails::user_id.eq(self.user_id))
                .filter(emails::is_primary.eq(true)),
        )
        .set((
            emails::email.eq(self.email),
            emails::verified.eq(self.verified),
        ))
        .returning(Email::as_returning())
        .get_result(conn)
        .await
        .optional()?;

        if let Some(email) = updated_email {
            Ok(email)
        } else {
            // Otherwise, insert a new email
            self.insert(conn).await
        }
    }
}
