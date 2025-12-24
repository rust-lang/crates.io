use crate::models::helpers::with_count::*;
use crate::models::version::TopVersions;
use crate::models::{CrateOwner, Owner, OwnerKind, ReverseDependency, User, Version};
use crate::schema::*;
use chrono::{DateTime, Utc};
use crates_io_diesel_helpers::canon_crate_name;
use diesel::associations::Identifiable;
use diesel::dsl;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Integer, Text};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use secrecy::SecretString;
use serde::Serialize;
use thiserror::Error;
use tracing::instrument;

use super::Team;

#[derive(Debug, Clone, HasQuery)]
#[diesel(table_name = crates)]
pub struct CrateName {
    pub name: String,
}

#[derive(Debug, Clone, Identifiable, AsChangeset, HasQuery, Serialize)]
#[diesel(table_name = crates)]
pub struct Crate {
    pub id: i32,
    pub name: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    pub max_upload_size: Option<i32>,
    pub max_features: Option<i16>,
    pub trustpub_only: bool,
}

/// We literally never want to select `textsearchable_index_col`
/// so we provide this type and constant to pass to `.select`
type AllColumns = (
    crates::id,
    crates::name,
    crates::updated_at,
    crates::created_at,
    crates::description,
    crates::homepage,
    crates::documentation,
    crates::repository,
    crates::max_upload_size,
    crates::max_features,
    crates::trustpub_only,
);

pub const ALL_COLUMNS: AllColumns = (
    crates::id,
    crates::name,
    crates::updated_at,
    crates::created_at,
    crates::description,
    crates::homepage,
    crates::documentation,
    crates::repository,
    crates::max_upload_size,
    crates::max_features,
    crates::trustpub_only,
);

type All = diesel::dsl::Select<crates::table, diesel::dsl::AsSelect<Crate, diesel::pg::Pg>>;
type WithName<'a> = diesel::dsl::Eq<canon_crate_name<crates::name>, canon_crate_name<&'a str>>;

#[derive(Insertable, AsChangeset, Default, Debug)]
#[diesel(
    table_name = crates,
    check_for_backend(diesel::pg::Pg),
    treat_none_as_null = true,
)]
pub struct NewCrate<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub homepage: Option<&'a str>,
    pub documentation: Option<&'a str>,
    pub readme: Option<&'a str>,
    pub repository: Option<&'a str>,
    pub max_upload_size: Option<i32>,
    pub max_features: Option<i16>,
}

impl NewCrate<'_> {
    pub async fn update(&self, conn: &mut AsyncPgConnection) -> QueryResult<Crate> {
        use diesel::update;

        update(crates::table)
            .filter(canon_crate_name(crates::name).eq(canon_crate_name(self.name)))
            .set((
                crates::description.eq(self.description),
                crates::homepage.eq(self.homepage),
                crates::documentation.eq(self.documentation),
                crates::readme.eq(self.readme),
                crates::repository.eq(self.repository),
            ))
            .returning(Crate::as_returning())
            .get_result(conn)
            .await
    }

    pub async fn create(&self, conn: &mut AsyncPgConnection, user_id: i32) -> QueryResult<Crate> {
        conn.transaction(|conn| {
            async move {
                let krate: Crate = diesel::insert_into(crates::table)
                    .values(self)
                    .on_conflict_do_nothing()
                    .returning(Crate::as_returning())
                    .get_result(conn)
                    .await?;

                CrateOwner::builder()
                    .crate_id(krate.id)
                    .user_id(user_id)
                    .created_by(user_id)
                    .build()
                    .insert(conn)
                    .await?;

                Ok(krate)
            }
            .scope_boxed()
        })
        .await
    }
}

impl Crate {
    pub fn max_upload_size(&self) -> Option<u32> {
        self.max_upload_size
            .and_then(|size| u32::try_from(size).ok())
    }

    /// SQL filter based on whether the crate's name loosely matches the given
    /// string.
    ///
    /// The operator used varies based on the input.
    pub fn loosly_matches_name<QS>(
        name: &str,
    ) -> Box<dyn BoxableExpression<QS, Pg, SqlType = Bool> + '_>
    where
        crates::name: SelectableExpression<QS>,
    {
        if name.len() > 2 {
            let wildcard_name = format!("%{name}%");
            Box::new(canon_crate_name(crates::name).like(canon_crate_name(wildcard_name)))
        } else {
            diesel::infix_operator!(MatchesWord, "%>");
            Box::new(MatchesWord::new(
                canon_crate_name(crates::name),
                name.into_sql::<Text>(),
            ))
        }
    }

    /// SQL filter with the = binary operator
    pub fn with_name(name: &str) -> WithName<'_> {
        canon_crate_name(crates::name).eq(canon_crate_name(name))
    }

    #[dsl::auto_type(no_type_alias)]
    pub fn by_name<'a>(name: &'a str) -> _ {
        let all: All = Crate::all();
        let filter: WithName<'a> = Self::with_name(name);
        all.filter(filter)
    }

    #[dsl::auto_type(no_type_alias)]
    pub fn by_exact_name<'a>(name: &'a str) -> _ {
        let all: All = Crate::all();
        all.filter(crates::name.eq(name))
    }

    pub fn all() -> All {
        crates::table.select(Self::as_select())
    }

    pub async fn find_version(
        &self,
        conn: &mut AsyncPgConnection,
        version: &str,
    ) -> QueryResult<Option<Version>> {
        Version::belonging_to(self)
            .filter(versions::num.eq(version))
            .select(Version::as_select())
            .first(conn)
            .await
            .optional()
    }

    /// Return both the newest (most recently updated) and
    /// highest version (in semver order) for the current crate,
    /// where all top versions are not yanked.
    pub async fn top_versions(&self, conn: &mut AsyncPgConnection) -> QueryResult<TopVersions> {
        Ok(TopVersions::from_date_version_pairs(
            Version::belonging_to(self)
                .filter(versions::yanked.eq(false))
                .select((versions::created_at, versions::num))
                .load(conn)
                .await?,
        ))
    }

    pub async fn owners(&self, conn: &mut AsyncPgConnection) -> QueryResult<Vec<Owner>> {
        let users = CrateOwner::by_owner_kind(OwnerKind::User)
            .filter(crate_owners::crate_id.eq(self.id))
            .order((crate_owners::owner_id, crate_owners::owner_kind))
            .inner_join(users::table)
            .select(User::as_select())
            .load(conn)
            .await?
            .into_iter()
            .map(Owner::User);

        let teams = CrateOwner::by_owner_kind(OwnerKind::Team)
            .filter(crate_owners::crate_id.eq(self.id))
            .order((crate_owners::owner_id, crate_owners::owner_kind))
            .inner_join(teams::table)
            .select(Team::as_select())
            .load(conn)
            .await?
            .into_iter()
            .map(Owner::Team);

        Ok(users.chain(teams).collect())
    }

    pub async fn owner_remove(
        &self,
        conn: &mut AsyncPgConnection,
        login: &str,
    ) -> Result<(), OwnerRemoveError> {
        let query = diesel::sql_query(
            r#"WITH crate_owners_with_login AS (
                SELECT
                    crate_owners.*,
                    CASE WHEN crate_owners.owner_kind = 1 THEN
                         teams.login
                    ELSE
                         users.gh_login
                    END AS login
                FROM crate_owners
                LEFT JOIN teams
                    ON crate_owners.owner_id = teams.id
                    AND crate_owners.owner_kind = 1
                LEFT JOIN users
                    ON crate_owners.owner_id = users.id
                    AND crate_owners.owner_kind = 0
                WHERE crate_owners.crate_id = $1
                    AND crate_owners.deleted = false
            )
            UPDATE crate_owners
            SET deleted = true
            FROM crate_owners_with_login
            WHERE crate_owners.crate_id = crate_owners_with_login.crate_id
                AND crate_owners.owner_id = crate_owners_with_login.owner_id
                AND crate_owners.owner_kind = crate_owners_with_login.owner_kind
                AND lower(crate_owners_with_login.login) = lower($2);"#,
        );

        let num_updated_rows = query
            .bind::<Integer, _>(self.id)
            .bind::<Text, _>(login)
            .execute(conn)
            .await?;

        if num_updated_rows == 0 {
            return Err(OwnerRemoveError::not_found(login));
        }

        Ok(())
    }

    /// Returns (dependency, dependent crate name, dependent crate downloads)
    #[instrument(skip_all, fields(krate.name = %self.name))]
    pub async fn reverse_dependencies(
        &self,
        conn: &mut AsyncPgConnection,
        offset: i64,
        limit: i64,
    ) -> QueryResult<(Vec<ReverseDependency>, i64)> {
        use diesel::sql_query;
        use diesel::sql_types::{BigInt, Integer};

        let rows: Vec<WithCount<ReverseDependency>> =
            sql_query(include_str!("krate_reverse_dependencies.sql"))
                .bind::<Integer, _>(self.id)
                .bind::<BigInt, _>(offset)
                .bind::<BigInt, _>(limit)
                .load(conn)
                .await?;

        Ok(rows.records_and_total())
    }
}

/// Details of a newly created invite.
#[derive(Debug)]
pub enum NewOwnerInvite {
    /// The invitee was a [`User`], and they must accept the invite through the
    /// UI or via the provided invite token.
    User(User, SecretString),

    /// The invitee was a [`Team`], and they were immediately added as an owner.
    Team(Team),
}

#[derive(Debug, Error)]
pub enum OwnerRemoveError {
    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),
    #[error("Could not find owner with login `{login}`")]
    NotFound { login: String },
}

impl OwnerRemoveError {
    pub fn not_found(login: &str) -> Self {
        let login = login.to_string();
        Self::NotFound { login }
    }
}
