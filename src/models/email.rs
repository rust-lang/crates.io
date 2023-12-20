use chrono::NaiveDateTime;
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
    pub token_generated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = emails, check_for_backend(diesel::pg::Pg))]
pub struct NewEmail<'a> {
    pub user_id: i32,
    pub email: &'a str,
}
