use crate::models::User;
use crate::schema::follows;

#[derive(Insertable, Queryable, Identifiable, Associations, Clone, Copy, Debug)]
#[diesel(belongs_to(User))]
#[diesel(primary_key(user_id, crate_id))]
#[diesel(table_name = follows, check_for_backend(diesel::pg::Pg))]
pub struct Follow {
    pub user_id: i32,
    pub crate_id: i32,
}
