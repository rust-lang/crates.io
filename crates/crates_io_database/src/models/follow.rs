use crate::models::User;
use crate::schema::follows;
use diesel::prelude::*;

#[derive(Insertable, Queryable, Identifiable, Associations, Clone, Copy, Debug)]
#[diesel(
    table_name = follows,
    check_for_backend(diesel::pg::Pg),
    primary_key(user_id, crate_id),
    belongs_to(User),
)]
pub struct Follow {
    pub user_id: i32,
    pub crate_id: i32,
}
