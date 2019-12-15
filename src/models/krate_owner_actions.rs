use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{
    deserialize::{self, FromSql},
    pg::Pg,
    serialize::{self, Output, ToSql},
    sql_types::Integer,
};
use std::io::Write;

use crate::models::{ApiToken, Crate, User};
use crate::schema::*;

#[derive(Debug, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[repr(i32)]
#[sql_type = "Integer"]
pub enum CrateAction {
    InviteUser = 0,
    RemoveUser = 1,
}

impl Into<&'static str> for CrateAction {
    fn into(self) -> &'static str {
        match self {
            CrateAction::InviteUser => "invite_user",
            CrateAction::RemoveUser => "remove_user",
        }
    }
}

impl Into<String> for CrateAction {
    fn into(self) -> String {
        let string: &'static str = self.into();

        string.into()
    }
}

impl FromSql<Integer, Pg> for CrateAction {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match <i32 as FromSql<Integer, Pg>>::from_sql(bytes)? {
            0 => Ok(CrateAction::InviteUser),
            1 => Ok(CrateAction::RemoveUser),
            n => Err(format!("unknown crate action: {}", n).into()),
        }
    }
}

impl ToSql<Integer, Pg> for CrateAction {
    fn to_sql<W: Write>(&self, out: &mut Output<'_, W, Pg>) -> serialize::Result {
        ToSql::<Integer, Pg>::to_sql(&(*self as i32), out)
    }
}

#[derive(Debug, Clone, Copy, Queryable, Identifiable, Associations)]
#[belongs_to(Crate)]
#[belongs_to(User)]
#[belongs_to(ApiToken)]
pub struct CrateOwnerAction {
    pub id: i32,
    pub crate_id: i32,
    pub user_id: i32,
    pub api_token_id: Option<i32>,
    pub action: CrateAction,
    pub time: NaiveDateTime,
}

impl CrateOwnerAction {
    pub fn all(conn: &PgConnection) -> QueryResult<Vec<Self>> {
        crate_owner_actions::table.load(conn)
    }

    pub fn by_crate(conn: &PgConnection, krate: &Crate) -> QueryResult<Vec<(Self, User)>> {
        use crate_owner_actions::dsl::crate_id;

        crate_owner_actions::table
            .filter(crate_id.eq(krate.id))
            .inner_join(users::table)
            .order(crate_owner_actions::dsl::id)
            .load(conn)
    }
}

pub fn insert_crate_owner_action(
    conn: &PgConnection,
    crate_id_: i32,
    user_id_: i32,
    api_token_id_: Option<i32>,
    action_: CrateAction,
) -> QueryResult<CrateOwnerAction> {
    use crate_owner_actions::dsl::{action, api_token_id, crate_id, user_id};

    diesel::insert_into(crate_owner_actions::table)
        .values((
            crate_id.eq(crate_id_),
            user_id.eq(user_id_),
            api_token_id.eq(api_token_id_),
            action.eq(action_),
        ))
        .get_result(conn)
}
