use crate::schema::persistent_sessions;
use chrono::NaiveDateTime;
use ipnetwork::IpNetwork;

#[derive(Clone, Debug, PartialEq, Eq, Identifiable, Queryable)]
#[table_name = "persistent_sessions"]
pub struct PersistentSession {
    pub id: i32,
    pub user_id: i32,
    pub hashed_token: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
    pub revoked: bool,
    pub last_ip_address: IpNetwork,
    pub last_user_agent: String,
}
