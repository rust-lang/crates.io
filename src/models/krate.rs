use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use diesel::associations::Identifiable;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Text};

use crate::app::App;
use crate::controllers::helpers::pagination::*;
use crate::models::version::TopVersions;
use crate::models::{
    CrateOwner, CrateOwnerInvitation, Dependency, NewCrateOwnerInvitationOutcome, Owner, OwnerKind,
    ReverseDependency, User, Version,
};
use crate::util::errors::{cargo_err, AppResult};

use crate::models::helpers::with_count::*;
use crate::schema::*;
use crate::sql::canon_crate_name;

#[derive(Debug, Queryable, Identifiable, Associations, Clone, Copy)]
#[diesel(
    table_name = recent_crate_downloads,
    check_for_backend(diesel::pg::Pg),
    primary_key(crate_id),
    belongs_to(Crate),
)]
pub struct RecentCrateDownloads {
    pub crate_id: i32,
    pub downloads: i32,
}

#[derive(Debug, Clone, Queryable, Identifiable, AsChangeset, QueryableByName, Selectable)]
#[diesel(table_name = crates, check_for_backend(diesel::pg::Pg))]
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
    pub max_features: Option<i16>,
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
    crates::max_features,
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
    crates::max_features,
);

pub const MAX_NAME_LENGTH: usize = 64;

type CanonCrateName<T> = canon_crate_name::HelperType<T>;
type All = diesel::dsl::Select<crates::table, diesel::dsl::AsSelect<Crate, diesel::pg::Pg>>;
type WithName<'a> = diesel::dsl::Eq<CanonCrateName<crates::name>, CanonCrateName<&'a str>>;
type ByName<'a> = diesel::dsl::Filter<All, WithName<'a>>;
type ByExactName<'a> = diesel::dsl::Filter<All, diesel::dsl::Eq<crates::name, &'a str>>;

#[derive(Insertable, AsChangeset, Default, Debug)]
#[diesel(
    table_name = crates,
    check_for_backend(diesel::pg::Pg),
    // This is actually just to skip updating them
    primary_key(name, max_upload_size),
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

impl<'a> NewCrate<'a> {
    pub fn update(&self, conn: &mut PgConnection) -> QueryResult<Crate> {
        use diesel::update;

        update(crates::table)
            .filter(canon_crate_name(crates::name).eq(canon_crate_name(self.name)))
            .set(self)
            .returning(Crate::as_returning())
            .get_result(conn)
    }

    pub fn create(&self, conn: &mut PgConnection, user_id: i32) -> QueryResult<Crate> {
        use crate::schema::crates::dsl::*;

        conn.transaction(|conn| {
            let krate: Crate = diesel::insert_into(crates)
                .values(self)
                .on_conflict_do_nothing()
                .returning(Crate::as_returning())
                .get_result(conn)?;

            let owner = CrateOwner {
                crate_id: krate.id,
                owner_id: user_id,
                created_by: user_id,
                owner_kind: OwnerKind::User,
                email_notifications: true,
            };

            diesel::insert_into(crate_owners::table)
                .values(&owner)
                .execute(conn)?;

            Ok(krate)
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

    pub fn by_name(name: &str) -> ByName<'_> {
        Crate::all().filter(Self::with_name(name))
    }

    pub fn by_exact_name(name: &str) -> ByExactName<'_> {
        Crate::all().filter(crates::name.eq(name))
    }

    pub fn all() -> All {
        crates::table.select(Self::as_select())
    }

    pub fn find_version(&self, conn: &mut PgConnection, version: &str) -> AppResult<Version> {
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
    pub fn top_versions(&self, conn: &mut PgConnection) -> QueryResult<TopVersions> {
        use crate::schema::versions::dsl::*;

        Ok(TopVersions::from_date_version_pairs(
            self.versions().select((created_at, num)).load(conn)?,
        ))
    }

    pub fn owners(&self, conn: &mut PgConnection) -> QueryResult<Vec<Owner>> {
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
        conn: &mut PgConnection,
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
                        owner_kind: OwnerKind::Team,
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

    pub fn owner_remove(&self, conn: &mut PgConnection, login: &str) -> AppResult<()> {
        let owner = Owner::find_by_login(conn, login)?;

        let target = crate_owners::table.find((self.id(), owner.id(), owner.kind()));
        diesel::update(target)
            .set(crate_owners::deleted.eq(true))
            .execute(conn)?;
        Ok(())
    }

    /// Returns (dependency, dependent crate name, dependent crate downloads)
    #[instrument(skip_all, fields(krate.name = %self.name))]
    pub(crate) fn reverse_dependencies(
        &self,
        conn: &mut PgConnection,
        options: PaginationOptions,
    ) -> AppResult<(Vec<ReverseDependency>, i64)> {
        use diesel::sql_query;
        use diesel::sql_types::{BigInt, Integer};

        let offset = options.offset().unwrap_or_default();
        let rows: Vec<WithCount<ReverseDependency>> =
            sql_query(include_str!("krate_reverse_dependencies.sql"))
                .bind::<Integer, _>(self.id)
                .bind::<BigInt, _>(offset)
                .bind::<BigInt, _>(options.per_page)
                .load(conn)?;

        Ok(rows.records_and_total())
    }

    /// Gather all the necessary data to write an index metadata file
    pub fn index_metadata(
        &self,
        conn: &mut PgConnection,
    ) -> QueryResult<Vec<crates_io_index::Crate>> {
        let mut versions: Vec<Version> = self.all_versions().load(conn)?;

        // We sort by `created_at` by default, but since tests run within a
        // single database transaction the versions will all have the same
        // `created_at` timestamp, so we sort by semver as a secondary key.
        versions.sort_by_cached_key(|k| (k.created_at, semver::Version::parse(&k.num).ok()));

        let deps: Vec<(Dependency, String)> = Dependency::belonging_to(&versions)
            .inner_join(crates::table)
            .select((dependencies::all_columns, crates::name))
            .load(conn)?;

        let deps = deps.grouped_by(&versions);

        versions
            .into_iter()
            .zip(deps)
            .map(|(version, deps)| {
                let mut deps = deps
                    .into_iter()
                    .map(|(dep, name)| {
                        // If this dependency has an explicit name in `Cargo.toml` that
                        // means that the `name` we have listed is actually the package name
                        // that we're depending on. The `name` listed in the index is the
                        // Cargo.toml-written-name which is what cargo uses for
                        // `--extern foo=...`
                        let (name, package) = match dep.explicit_name {
                            Some(explicit_name) => (explicit_name, Some(name)),
                            None => (name, None),
                        };

                        crates_io_index::Dependency {
                            name,
                            req: dep.req,
                            features: dep.features,
                            optional: dep.optional,
                            default_features: dep.default_features,
                            kind: Some(dep.kind.into()),
                            package,
                            target: dep.target,
                        }
                    })
                    .collect::<Vec<_>>();

                deps.sort();

                let features: BTreeMap<String, Vec<String>> =
                    serde_json::from_value(version.features).unwrap_or_default();
                let (features, features2): (BTreeMap<_, _>, BTreeMap<_, _>) =
                    features.into_iter().partition(|(_k, vals)| {
                        !vals
                            .iter()
                            .any(|v| v.starts_with("dep:") || v.contains("?/"))
                    });

                let (features2, v) = if features2.is_empty() {
                    (None, None)
                } else {
                    (Some(features2), Some(2))
                };

                let krate = crates_io_index::Crate {
                    name: self.name.clone(),
                    vers: version.num.to_string(),
                    cksum: version.checksum,
                    yanked: Some(version.yanked),
                    deps,
                    features,
                    links: version.links,
                    rust_version: version.rust_version,
                    features2,
                    v,
                };

                Ok(krate)
            })
            .collect()
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

#[cfg(test)]
mod tests {
    use crate::models::Crate;

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
