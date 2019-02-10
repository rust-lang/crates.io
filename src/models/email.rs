use chrono::NaiveDateTime;

use crate::models::User;
use crate::schema::emails;

#[derive(Debug, Queryable, AsChangeset, Identifiable, Associations)]
#[belongs_to(User)]
pub struct Email {
    pub id: i32,
    pub user_id: i32,
    pub email: String,
    pub verified: bool,
    pub token: String,
    pub token_generated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Insertable, AsChangeset)]
#[table_name = "emails"]
pub struct NewEmail<'a> {
    pub user_id: i32,
    pub email: &'a str,
}
