use crate::models::User;
use crate::schema::follows;

#[derive(Insertable, Queryable, Identifiable, Associations, Clone, Copy, Debug)]
#[belongs_to(User)]
#[primary_key(user_id, crate_id)]
#[table_name = "follows"]
pub struct Follow {
    pub user_id: i32,
    pub crate_id: i32,
}
