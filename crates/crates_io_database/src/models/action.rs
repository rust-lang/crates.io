use crate::models::{ApiToken, User, Version};
use crate::schema::*;
use bon::Builder;
use chrono::{DateTime, Utc};
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

#[derive(Debug, Clone, Copy, HasQuery, Identifiable, Associations)]
#[diesel(
    table_name = version_owner_actions,
    belongs_to(Version),
    belongs_to(User, foreign_key = user_id),
    belongs_to(ApiToken, foreign_key = api_token_id),
    belongs_to(crate::models::download::Version, foreign_key = version_id),
)]
pub struct VersionOwnerAction {
    pub id: i32,
    pub version_id: i32,
    pub user_id: i32,
    pub api_token_id: Option<i32>,
    pub action: VersionAction,
    pub time: DateTime<Utc>,
}

impl VersionOwnerAction {
    pub async fn all(mut conn: &AsyncPgConnection) -> QueryResult<Vec<Self>> {
        Self::query().load(&mut conn).await
    }

    pub async fn by_version(
        mut conn: &AsyncPgConnection,
        version: &Version,
    ) -> QueryResult<Vec<(Self, User)>> {
        use version_owner_actions::dsl::version_id;

        version_owner_actions::table
            .filter(version_id.eq(version.id))
            .inner_join(users::table)
            .select((VersionOwnerAction::as_select(), User::as_select()))
            .order(version_owner_actions::dsl::id)
            .load(&mut conn)
            .await
    }

    pub async fn for_versions(
        mut conn: &AsyncPgConnection,
        versions: &[&Version],
    ) -> QueryResult<Vec<Vec<(Self, User)>>> {
        Ok(Self::belonging_to(versions)
            .inner_join(users::table)
            .select((VersionOwnerAction::as_select(), User::as_select()))
            .order(version_owner_actions::dsl::id)
            .load(&mut conn)
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
    pub async fn insert(&self, mut conn: &AsyncPgConnection) -> QueryResult<VersionOwnerAction> {
        diesel::insert_into(version_owner_actions::table)
            .values(self)
            .returning(VersionOwnerAction::as_select())
            .get_result(&mut conn)
            .await
    }
}
