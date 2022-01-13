use chrono::NaiveDateTime;
use diesel::associations::Identifiable;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Text};
use url::Url;

use crate::app::App;
use crate::controllers::helpers::pagination::*;
use crate::models::version::TopVersions;
use crate::models::{
    Badge, CrateOwner, CrateOwnerInvitation, NewCrateOwnerInvitationOutcome, Owner, OwnerKind,
    ReverseDependency, User, Version,
};
use crate::util::errors::{cargo_err, AppResult};

use crate::models::helpers::with_count::*;
use crate::publish_rate_limit::PublishRateLimit;
use crate::schema::*;
use crate::sql::canon_crate_name;

#[derive(Debug, Queryable, Identifiable, Associations, Clone, Copy)]
#[belongs_to(Crate)]
#[primary_key(crate_id)]
#[table_name = "recent_crate_downloads"]
pub struct RecentCrateDownloads {
    pub crate_id: i32,
    pub downloads: i32,
}

#[derive(Debug, Clone, Queryable, Identifiable, Associations, AsChangeset, QueryableByName)]
#[table_name = "crates"]
pub struct Crate {
    pub id: i32,
    pub name: String,
    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub downloads: i32,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    pub max_upload_size: Option<i32>,
}

/// We literally never want to select `textsearchable_index_col`
/// so we provide this type and constant to pass to `.select`
type AllColumns = (
    crates::id,
    crates::name,
    crates::updated_at,
    crates::created_at,
    crates::downloads,
    crates::description,
    crates::homepage,
    crates::documentation,
    crates::repository,
    crates::max_upload_size,
);

pub const ALL_COLUMNS: AllColumns = (
    crates::id,
    crates::name,
    crates::updated_at,
    crates::created_at,
    crates::downloads,
    crates::description,
    crates::homepage,
    crates::documentation,
    crates::repository,
    crates::max_upload_size,
);

pub const MAX_NAME_LENGTH: usize = 64;

type CanonCrateName<T> = canon_crate_name::HelperType<T>;
type All = diesel::dsl::Select<crates::table, AllColumns>;
type WithName<'a> = diesel::dsl::Eq<CanonCrateName<crates::name>, CanonCrateName<&'a str>>;
type ByName<'a> = diesel::dsl::Filter<All, WithName<'a>>;
type ByExactName<'a> = diesel::dsl::Filter<All, diesel::dsl::Eq<crates::name, &'a str>>;

#[derive(Insertable, AsChangeset, Default, Debug)]
#[table_name = "crates"]
#[changeset_options(treat_none_as_null = "true")]
#[primary_key(name, max_upload_size)] // This is actually just to skip updating them
pub struct NewCrate<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub homepage: Option<&'a str>,
    pub documentation: Option<&'a str>,
    pub readme: Option<&'a str>,
    pub repository: Option<&'a str>,
    pub max_upload_size: Option<i32>,
}

impl<'a> NewCrate<'a> {
    pub fn create_or_update(
        self,
        conn: &PgConnection,
        uploader: i32,
        rate_limit: Option<&PublishRateLimit>,
    ) -> AppResult<Crate> {
        use diesel::update;

        self.validate()?;
        self.ensure_name_not_reserved(conn)?;

        conn.transaction(|| {
            // To avoid race conditions, we try to insert
            // first so we know whether to add an owner
            if let Some(krate) = self.save_new_crate(conn, uploader)? {
                if let Some(rate_limit) = rate_limit {
                    rate_limit.check_rate_limit(uploader, conn)?;
                }
                return Ok(krate);
            }

            update(crates::table)
                .filter(canon_crate_name(crates::name).eq(canon_crate_name(self.name)))
                .set(&self)
                .returning(ALL_COLUMNS)
                .get_result(conn)
                .map_err(Into::into)
        })
    }

    fn validate(&self) -> AppResult<()> {
        fn validate_url(url: Option<&str>, field: &str) -> AppResult<()> {
            let url = match url {
                Some(s) => s,
                None => return Ok(()),
            };

            // Manually check the string, as `Url::parse` may normalize relative URLs
            // making it difficult to ensure that both slashes are present.
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(cargo_err(&format_args!(
                    "URL for field `{}` must begin with http:// or https:// (url: {})",
                    field, url
                )));
            }

            // Ensure the entire URL parses as well
            Url::parse(url).map_err(|_| {
                cargo_err(&format_args!("`{}` is not a valid url: `{}`", field, url))
            })?;
            Ok(())
        }

        validate_url(self.homepage, "homepage")?;
        validate_url(self.documentation, "documentation")?;
        validate_url(self.repository, "repository")?;
        Ok(())
    }

    fn ensure_name_not_reserved(&self, conn: &PgConnection) -> AppResult<()> {
        use crate::schema::reserved_crate_names::dsl::*;
        use diesel::dsl::exists;
        use diesel::select;

        let reserved_name: bool = select(exists(
            reserved_crate_names.filter(canon_crate_name(name).eq(canon_crate_name(self.name))),
        ))
        .get_result(conn)?;
        if reserved_name {
            Err(cargo_err("cannot upload a crate with a reserved name"))
        } else {
            Ok(())
        }
    }

    fn save_new_crate(&self, conn: &PgConnection, user_id: i32) -> QueryResult<Option<Crate>> {
        use crate::schema::crates::dsl::*;

        conn.transaction(|| {
            let maybe_inserted: Option<Crate> = diesel::insert_into(crates)
                .values(self)
                .on_conflict_do_nothing()
                .returning(ALL_COLUMNS)
                .get_result(conn)
                .optional()?;

            if let Some(ref krate) = maybe_inserted {
                let owner = CrateOwner {
                    crate_id: krate.id,
                    owner_id: user_id,
                    created_by: user_id,
                    owner_kind: OwnerKind::User as i32,
                    email_notifications: true,
                };
                diesel::insert_into(crate_owners::table)
                    .values(&owner)
                    .execute(conn)?;
            }

            Ok(maybe_inserted)
        })
    }
}

impl Crate {
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
            diesel_infix_operator!(MatchesWord, "%>");
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

    pub fn by_name(name: &str) -> ByName<'_> {
        Crate::all().filter(Self::with_name(name))
    }

    pub fn by_exact_name(name: &str) -> ByExactName<'_> {
        Crate::all().filter(crates::name.eq(name))
    }

    pub fn all() -> All {
        crates::table.select(ALL_COLUMNS)
    }

    pub fn find_version(&self, conn: &PgConnection, version: &str) -> AppResult<Version> {
        self.all_versions()
            .filter(versions::num.eq(version))
            .first(conn)
            .map_err(|_| {
                cargo_err(&format_args!(
                    "crate `{}` does not have a version `{}`",
                    self.name, version
                ))
            })
    }

    pub fn valid_name(name: &str) -> bool {
        let under_max_length = name.chars().take(MAX_NAME_LENGTH + 1).count() <= MAX_NAME_LENGTH;
        Crate::valid_ident(name) && under_max_length
    }

    fn valid_ident(name: &str) -> bool {
        Self::valid_feature_prefix(name)
            && name
                .chars()
                .next()
                .map(char::is_alphabetic)
                .unwrap_or(false)
    }

    pub fn valid_dependency_name(name: &str) -> bool {
        let under_max_length = name.chars().take(MAX_NAME_LENGTH + 1).count() <= MAX_NAME_LENGTH;
        Crate::valid_dependency_ident(name) && under_max_length
    }

    fn valid_dependency_ident(name: &str) -> bool {
        Self::valid_feature_prefix(name)
            && name
                .chars()
                .next()
                .map(|n| n.is_alphabetic() || n == '_')
                .unwrap_or(false)
    }

    /// Validates the THIS parts of `features = ["THIS", "and/THIS"]`.
    pub fn valid_feature_name(name: &str) -> bool {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '+')
    }

    /// Validates the prefix in front of the slash: `features = ["THIS/feature"]`.
    /// Normally this corresponds to the crate name of a dependency.
    fn valid_feature_prefix(name: &str) -> bool {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    }

    /// Validates a whole feature string, `features = ["THIS", "ALL/THIS"]`.
    pub fn valid_feature(name: &str) -> bool {
        match name.split_once('/') {
            Some((dep, dep_feat)) => {
                let dep = dep.strip_suffix('?').unwrap_or(dep);
                Crate::valid_feature_prefix(dep) && Crate::valid_feature_name(dep_feat)
            }
            None => Crate::valid_feature_name(name.strip_prefix("dep:").unwrap_or(name)),
        }
    }

    /// Return both the newest (most recently updated) and
    /// highest version (in semver order) for the current crate.
    pub fn top_versions(&self, conn: &PgConnection) -> QueryResult<TopVersions> {
        use crate::schema::versions::dsl::*;

        Ok(TopVersions::from_date_version_pairs(
            self.versions().select((created_at, num)).load(conn)?,
        ))
    }

    pub fn owners(&self, conn: &PgConnection) -> QueryResult<Vec<Owner>> {
        let users = CrateOwner::by_owner_kind(OwnerKind::User)
            .filter(crate_owners::crate_id.eq(self.id))
            .inner_join(users::table)
            .select(users::all_columns)
            .load(conn)?
            .into_iter()
            .map(Owner::User);
        let teams = CrateOwner::by_owner_kind(OwnerKind::Team)
            .filter(crate_owners::crate_id.eq(self.id))
            .inner_join(teams::table)
            .select(teams::all_columns)
            .load(conn)?
            .into_iter()
            .map(Owner::Team);

        Ok(users.chain(teams).collect())
    }

    pub fn owner_add(
        &self,
        app: &App,
        conn: &PgConnection,
        req_user: &User,
        login: &str,
    ) -> AppResult<String> {
        use diesel::insert_into;

        let owner = Owner::find_or_create_by_login(app, conn, req_user, login)?;

        match owner {
            // Users are invited and must accept before being added
            Owner::User(user) => {
                let config = &app.config;
                match CrateOwnerInvitation::create(user.id, req_user.id, self.id, conn, config)? {
                    NewCrateOwnerInvitationOutcome::InviteCreated { plaintext_token } => {
                        if let Ok(Some(email)) = user.verified_email(conn) {
                            // Swallow any error. Whether or not the email is sent, the invitation
                            // entry will be created in the database and the user will see the
                            // invitation when they visit https://crates.io/me/pending-invites/.
                            let _ = app.emails.send_owner_invite(
                                &email,
                                &req_user.gh_login,
                                &self.name,
                                &plaintext_token,
                            );
                        }

                        Ok(format!(
                            "user {} has been invited to be an owner of crate {}",
                            user.gh_login, self.name
                        ))
                    }
                    NewCrateOwnerInvitationOutcome::AlreadyExists => Ok(format!(
                        "user {} already has a pending invitation to be an owner of crate {}",
                        user.gh_login, self.name
                    )),
                }
            }
            // Teams are added as owners immediately
            owner @ Owner::Team(_) => {
                insert_into(crate_owners::table)
                    .values(&CrateOwner {
                        crate_id: self.id,
                        owner_id: owner.id(),
                        created_by: req_user.id,
                        owner_kind: OwnerKind::Team as i32,
                        email_notifications: true,
                    })
                    .on_conflict(crate_owners::table.primary_key())
                    .do_update()
                    .set(crate_owners::deleted.eq(false))
                    .execute(conn)?;

                Ok(format!(
                    "team {} has been added as an owner of crate {}",
                    owner.login(),
                    self.name
                ))
            }
        }
    }

    pub fn owner_remove(
        &self,
        app: &App,
        conn: &PgConnection,
        req_user: &User,
        login: &str,
    ) -> AppResult<()> {
        let owner = Owner::find_or_create_by_login(app, conn, req_user, login)?;

        let target = crate_owners::table.find((self.id(), owner.id(), owner.kind() as i32));
        diesel::update(target)
            .set(crate_owners::deleted.eq(true))
            .execute(conn)?;
        Ok(())
    }

    pub fn badges(&self, conn: &PgConnection) -> QueryResult<Vec<Badge>> {
        badges::table
            .filter(badges::crate_id.eq(self.id))
            .load(conn)
    }

    /// Returns (dependency, dependent crate name, dependent crate downloads)
    pub(crate) fn reverse_dependencies(
        &self,
        conn: &PgConnection,
        options: PaginationOptions,
    ) -> AppResult<(Vec<ReverseDependency>, i64)> {
        use diesel::sql_query;
        use diesel::sql_types::{BigInt, Integer};

        let offset = options.offset().unwrap_or_default();
        let rows: Vec<WithCount<ReverseDependency>> =
            sql_query(include_str!("krate_reverse_dependencies.sql"))
                .bind::<Integer, _>(self.id)
                .bind::<BigInt, _>(i64::from(offset))
                .bind::<BigInt, _>(i64::from(options.per_page))
                .load(conn)?;

        Ok(rows.records_and_total())
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{Crate, NewCrate};

    #[test]
    fn deny_relative_urls() {
        let krate = NewCrate {
            name: "name",
            description: None,
            homepage: Some("https:/example.com/home"),
            documentation: None,
            readme: None,
            repository: None,
            max_upload_size: None,
        };
        assert_err!(krate.validate());
    }

    #[test]
    fn valid_name() {
        assert!(Crate::valid_name("foo"));
        assert!(!Crate::valid_name("京"));
        assert!(!Crate::valid_name(""));
        assert!(!Crate::valid_name("💝"));
        assert!(Crate::valid_name("foo_underscore"));
        assert!(Crate::valid_name("foo-dash"));
        assert!(!Crate::valid_name("foo+plus"));
        // Starting with an underscore is an invalid crate name.
        assert!(!Crate::valid_name("_foo"));
        assert!(!Crate::valid_name("-foo"));
    }

    #[test]
    fn valid_dependency_name() {
        assert!(Crate::valid_dependency_name("foo"));
        assert!(!Crate::valid_dependency_name("京"));
        assert!(!Crate::valid_dependency_name(""));
        assert!(!Crate::valid_dependency_name("💝"));
        assert!(Crate::valid_dependency_name("foo_underscore"));
        assert!(Crate::valid_dependency_name("foo-dash"));
        assert!(!Crate::valid_dependency_name("foo+plus"));
        // Starting with an underscore is a valid dependency name.
        assert!(Crate::valid_dependency_name("_foo"));
        assert!(!Crate::valid_dependency_name("-foo"));
    }

    #[test]
    fn valid_feature_names() {
        assert!(Crate::valid_feature("foo"));
        assert!(!Crate::valid_feature(""));
        assert!(!Crate::valid_feature("/"));
        assert!(!Crate::valid_feature("%/%"));
        assert!(Crate::valid_feature("a/a"));
        assert!(Crate::valid_feature("32-column-tables"));
        assert!(Crate::valid_feature("c++20"));
        assert!(Crate::valid_feature("krate/c++20"));
        assert!(!Crate::valid_feature("c++20/wow"));
        assert!(Crate::valid_feature("foo?/bar"));
        assert!(Crate::valid_feature("dep:foo"));
        assert!(!Crate::valid_feature("dep:foo?/bar"));
        assert!(!Crate::valid_feature("foo/?bar"));
        assert!(!Crate::valid_feature("foo?bar"));
    }
}

pub trait CrateVersions {
    fn versions(&self) -> versions::BoxedQuery<'_, Pg> {
        self.all_versions().filter(versions::yanked.eq(false))
    }

    fn all_versions(&self) -> versions::BoxedQuery<'_, Pg>;
}

impl CrateVersions for Crate {
    fn all_versions(&self) -> versions::BoxedQuery<'_, Pg> {
        Version::belonging_to(self).into_boxed()
    }
}

impl CrateVersions for Vec<Crate> {
    fn all_versions(&self) -> versions::BoxedQuery<'_, Pg> {
        self.as_slice().all_versions()
    }
}

impl CrateVersions for [Crate] {
    fn all_versions(&self) -> versions::BoxedQuery<'_, Pg> {
        Version::belonging_to(self).into_boxed()
    }
}
