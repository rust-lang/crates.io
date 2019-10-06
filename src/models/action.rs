use chrono::NaiveDateTime;

use crate::models::{ApiToken, User, Version};
use crate::schema::*;

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum VersionAction {
    Publish = 0,
    Yank = 1,
    Unyank = 2,
}

#[derive(Debug, Clone, Copy, Queryable, Identifiable, Associations)]
#[belongs_to(Version)]
#[belongs_to(User, foreign_key = "owner_id")]
#[belongs_to(ApiToken, foreign_key = "owner_token_id")]
#[table_name = "version_owner_actions"]
pub struct VersionOwnerAction {
    pub id: i32,
    pub version_id: i32,
    pub owner_id: i32,
    pub owner_token_id: i32,
    pub action: VersionAction,
    pub time: NaiveDateTime,
}
