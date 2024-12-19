use crate::models::{ApiToken, User, Version};
use crate::schema::*;
use bon::Builder;
use chrono::NaiveDateTime;
use crates_io_diesel_helpers::pg_enum;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

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
    pub async fn all(conn: &mut AsyncPgConnection) -> QueryResult<Vec<Self>> {
        version_owner_actions::table.load(conn).await
    }

    pub async fn by_version(
        conn: &mut AsyncPgConnection,
        version: &Version,
    ) -> QueryResult<Vec<(Self, User)>> {
        use version_owner_actions::dsl::version_id;

        version_owner_actions::table
            .filter(version_id.eq(version.id))
            .inner_join(users::table)
            .order(version_owner_actions::dsl::id)
            .load(conn)
            .await
    }

    pub async fn for_versions(
        conn: &mut AsyncPgConnection,
        versions: &[&Version],
    ) -> QueryResult<Vec<Vec<(Self, User)>>> {
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
    pub async fn insert(&self, conn: &mut AsyncPgConnection) -> QueryResult<VersionOwnerAction> {
        diesel::insert_into(version_owner_actions::table)
            .values(self)
            .get_result(conn)
            .await
    }
}
