use crate::models::{ApiToken, User, Version};
use crate::schema::*;
use crate::sql::pg_enum;
use crate::util::diesel::prelude::*;
use crate::util::diesel::Conn;
use bon::Builder;
use chrono::NaiveDateTime;
use diesel_async::AsyncPgConnection;

pg_enum! {
    pub enum VersionAction {
        Publish = 0,
        Yank = 1,
        Unyank = 2,
    }
}

impl From<VersionAction> for &'static str {
    fn from(action: VersionAction) -> Self {
        match action {
            VersionAction::Publish => "publish",
            VersionAction::Yank => "yank",
            VersionAction::Unyank => "unyank",
        }
    }
}

impl From<VersionAction> for String {
    fn from(action: VersionAction) -> Self {
        let string: &'static str = action.into();

        string.into()
    }
}

#[derive(Debug, Clone, Copy, Queryable, Identifiable, Associations)]
#[diesel(
    table_name = version_owner_actions,
    check_for_backend(diesel::pg::Pg),
    belongs_to(Version),
    belongs_to(User, foreign_key = user_id),
    belongs_to(ApiToken, foreign_key = api_token_id),
)]
pub struct VersionOwnerAction {
    pub id: i32,
    pub version_id: i32,
    pub user_id: i32,
    pub api_token_id: Option<i32>,
    pub action: VersionAction,
    pub time: NaiveDateTime,
}

impl VersionOwnerAction {
    pub fn all(conn: &mut impl Conn) -> QueryResult<Vec<Self>> {
        use diesel::RunQueryDsl;

        version_owner_actions::table.load(conn)
    }

    pub fn by_version(conn: &mut impl Conn, version: &Version) -> QueryResult<Vec<(Self, User)>> {
        use diesel::RunQueryDsl;
        use version_owner_actions::dsl::version_id;

        version_owner_actions::table
            .filter(version_id.eq(version.id))
            .inner_join(users::table)
            .order(version_owner_actions::dsl::id)
            .load(conn)
    }

    pub fn for_versions(
        conn: &mut impl Conn,
        versions: &[&Version],
    ) -> QueryResult<Vec<Vec<(Self, User)>>> {
        use diesel::RunQueryDsl;

        Ok(Self::belonging_to(versions)
            .inner_join(users::table)
            .order(version_owner_actions::dsl::id)
            .load(conn)?
            .grouped_by(versions))
    }

    pub async fn async_for_versions(
        conn: &mut AsyncPgConnection,
        versions: &[&Version],
    ) -> QueryResult<Vec<Vec<(Self, User)>>> {
        use diesel_async::RunQueryDsl;

        Ok(Self::belonging_to(versions)
            .inner_join(users::table)
            .order(version_owner_actions::dsl::id)
            .load(conn)
            .await?
            .grouped_by(versions))
    }
}

#[derive(Insertable, Debug, Builder)]
#[diesel(table_name = version_owner_actions, check_for_backend(diesel::pg::Pg))]
pub struct NewVersionOwnerAction {
    version_id: i32,
    user_id: i32,
    api_token_id: Option<i32>,
    #[builder(into)]
    action: VersionAction,
}

impl NewVersionOwnerAction {
    pub fn insert(&self, conn: &mut impl Conn) -> QueryResult<VersionOwnerAction> {
        use diesel::RunQueryDsl;

        diesel::insert_into(version_owner_actions::table)
            .values(self)
            .get_result(conn)
    }

    pub async fn async_insert(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<VersionOwnerAction> {
        use diesel_async::RunQueryDsl;

        diesel::insert_into(version_owner_actions::table)
            .values(self)
            .get_result(conn)
            .await
    }
}
