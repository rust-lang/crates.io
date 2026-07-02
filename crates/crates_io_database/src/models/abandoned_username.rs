use crate::schema::abandoned_usernames;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

#[derive(Debug, Clone, Identifiable, HasQuery)]
#[diesel(
    table_name = abandoned_usernames,
    check_for_backend(diesel::pg::Pg),
)]
pub struct AbandonedUsername {
    pub id: i64,
    pub username: String,
    pub previous_user_id: Option<i32>,
    pub adopted_at: Option<DateTime<Utc>>,
    pub abandoned_at: DateTime<Utc>,
    pub available_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = abandoned_usernames, check_for_backend(diesel::pg::Pg))]
pub struct NewAbandonedUsername<'a> {
    pub username: &'a str,
    pub previous_user_id: Option<i32>,
    pub adopted_at: Option<&'a DateTime<Utc>>,
    pub abandoned_at: &'a DateTime<Utc>,
    pub available_at: &'a DateTime<Utc>,
}
