use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use diesel::associations::Identifiable;
use diesel::dsl;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Text};
use secrecy::{ExposeSecret, SecretString};

use crate::app::App;
use crate::controllers::helpers::pagination::*;
use crate::email::Email;
use crate::models::version::TopVersions;
use crate::models::{
    CrateOwner, CrateOwnerInvitation, Dependency, NewCrateOwnerInvitationOutcome, Owner, OwnerKind,
    ReverseDependency, User, Version,
};
use crate::util::errors::{version_not_found, AppResult};

use crate::models::helpers::with_count::*;
use crate::schema::*;
use crate::sql::canon_crate_name;
use crate::util::diesel::Conn;

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
    crates::description,
    crates::homepage,
    crates::documentation,
    crates::repository,
    crates::max_upload_size,
    crates::max_features,
);

pub const MAX_NAME_LENGTH: usize = 64;

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

impl<'a> NewCrate<'a> {
    pub fn update(&self, conn: &mut impl Conn) -> QueryResult<Crate> {
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
    }

    pub fn create(&self, conn: &mut impl Conn, user_id: i32) -> QueryResult<Crate> {
        conn.transaction(|conn| {
            let krate: Crate = diesel::insert_into(crates::table)
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

    pub fn find_version(&self, conn: &mut impl Conn, version: &str) -> AppResult<Version> {
        self.all_versions()
            .filter(versions::num.eq(version))
            .first(conn)
            .optional()?
            .ok_or_else(|| version_not_found(&self.name, version))
    }

    // Validates the name is a valid crate name.
    // This is also used for validating the name of dependencies.
    // So the `for_what` parameter is used to indicate what the name is used for.
    // It can be "crate" or "dependency".
    pub fn validate_crate_name(for_what: &str, name: &str) -> Result<(), InvalidCrateName> {
        if name.chars().count() > MAX_NAME_LENGTH {
            return Err(InvalidCrateName::TooLong {
                what: for_what.into(),
                name: name.into(),
            });
        }
        Crate::validate_create_ident(for_what, name)
    }

    // Checks that the name is a valid crate name.
    // 1. The name must be non-empty.
    // 2. The first character must be an ASCII character.
    // 3. The remaining characters must be ASCII alphanumerics or `-` or `_`.
    // Note: This differs from `valid_dependency_name`, which allows `_` as the first character.
    fn validate_create_ident(for_what: &str, name: &str) -> Result<(), InvalidCrateName> {
        if name.is_empty() {
            return Err(InvalidCrateName::Empty {
                what: for_what.into(),
            });
        }
        let mut chars = name.chars();
        if let Some(ch) = chars.next() {
            if ch.is_ascii_digit() {
                return Err(InvalidCrateName::StartWithDigit {
                    what: for_what.into(),
                    name: name.into(),
                });
            }
            if !ch.is_ascii_alphabetic() {
                return Err(InvalidCrateName::Start {
                    first_char: ch,
                    what: for_what.into(),
                    name: name.into(),
                });
            }
        }

        for ch in chars {
            if !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') {
                return Err(InvalidCrateName::Char {
                    ch,
                    what: for_what.into(),
                    name: name.into(),
                });
            }
        }

        Ok(())
    }

    pub fn validate_dependency_name(name: &str) -> Result<(), InvalidDependencyName> {
        if name.chars().count() > MAX_NAME_LENGTH {
            return Err(InvalidDependencyName::TooLong(name.into()));
        }
        Crate::validate_dependency_ident(name)
    }

    // Checks that the name is a valid dependency name.
    // 1. The name must be non-empty.
    // 2. The first character must be an ASCII character or `_`.
    // 3. The remaining characters must be ASCII alphanumerics or `-` or `_`.
    fn validate_dependency_ident(name: &str) -> Result<(), InvalidDependencyName> {
        if name.is_empty() {
            return Err(InvalidDependencyName::Empty);
        }
        let mut chars = name.chars();
        if let Some(ch) = chars.next() {
            if ch.is_ascii_digit() {
                return Err(InvalidDependencyName::StartWithDigit(name.into()));
            }
            if !(ch.is_ascii_alphabetic() || ch == '_') {
                return Err(InvalidDependencyName::Start(ch, name.into()));
            }
        }

        for ch in chars {
            if !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') {
                return Err(InvalidDependencyName::Char(ch, name.into()));
            }
        }

        Ok(())
    }

    /// Validates the THIS parts of `features = ["THIS", "and/THIS", "dep:THIS", "dep?/THIS"]`.
    /// 1. The name must be non-empty.
    /// 2. The first character must be a Unicode XID start character, `_`, or a digit.
    /// 3. The remaining characters must be Unicode XID characters, `_`, `+`, `-`, or `.`.
    pub fn validate_feature_name(name: &str) -> Result<(), InvalidFeature> {
        if name.is_empty() {
            return Err(InvalidFeature::Empty);
        }
        let mut chars = name.chars();
        if let Some(ch) = chars.next() {
            if !(unicode_xid::UnicodeXID::is_xid_start(ch) || ch == '_' || ch.is_ascii_digit()) {
                return Err(InvalidFeature::Start(ch, name.into()));
            }
        }
        for ch in chars {
            if !(unicode_xid::UnicodeXID::is_xid_continue(ch)
                || ch == '+'
                || ch == '-'
                || ch == '.')
            {
                return Err(InvalidFeature::Char(ch, name.into()));
            }
        }

        Ok(())
    }

    /// Validates a whole feature string, `features = ["THIS", "and/THIS", "dep:THIS", "dep?/THIS"]`.
    pub fn validate_feature(name: &str) -> Result<(), InvalidFeature> {
        if let Some((dep, dep_feat)) = name.split_once('/') {
            let dep = dep.strip_suffix('?').unwrap_or(dep);
            Crate::validate_dependency_name(dep)?;
            Crate::validate_feature_name(dep_feat)
        } else if let Some((_, dep)) = name.split_once("dep:") {
            Crate::validate_dependency_name(dep)?;
            return Ok(());
        } else {
            Crate::validate_feature_name(name)
        }
    }

    /// Return both the newest (most recently updated) and
    /// highest version (in semver order) for the current crate.
    pub fn top_versions(&self, conn: &mut impl Conn) -> QueryResult<TopVersions> {
        Ok(TopVersions::from_date_version_pairs(
            self.versions()
                .select((versions::created_at, versions::num))
                .load(conn)?,
        ))
    }

    pub fn owners(&self, conn: &mut impl Conn) -> QueryResult<Vec<Owner>> {
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

    /// Invite `login` as an owner of this crate, returning a status message and
    /// optionally an invite email to be sent by the caller.
    pub fn owner_add(
        &self,
        app: &App,
        conn: &mut impl Conn,
        req_user: &User,
        login: &str,
    ) -> AppResult<(String, Option<OwnerInviteEmail>)> {
        use diesel::insert_into;

        let owner = Owner::find_or_create_by_login(app, conn, req_user, login)?;

        match owner {
            // Users are invited and must accept before being added
            Owner::User(user) => {
                let config = &app.config;
                match CrateOwnerInvitation::create(user.id, req_user.id, self.id, conn, config)? {
                    NewCrateOwnerInvitationOutcome::InviteCreated { plaintext_token } => {
                        let email = user.verified_email(conn).ok().flatten().map(|recipient| {
                            OwnerInviteEmail {
                                recipient_email_address: recipient,
                                user_name: req_user.gh_login.clone(),
                                domain: app.emails.domain.clone(),
                                crate_name: self.name.clone(),
                                token: plaintext_token,
                            }
                        });

                        let msg = format!(
                            "user {} has been invited to be an owner of crate {}",
                            user.gh_login, self.name
                        );

                        Ok((msg, email))
                    }
                    NewCrateOwnerInvitationOutcome::AlreadyExists => Ok((
                        format!(
                            "user {} already has a pending invitation to be an owner of crate {}",
                            user.gh_login, self.name
                        ),
                        None,
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

                Ok((
                    format!(
                        "team {} has been added as an owner of crate {}",
                        owner.login(),
                        self.name
                    ),
                    None,
                ))
            }
        }
    }

    pub fn owner_remove(&self, conn: &mut impl Conn, login: &str) -> AppResult<()> {
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
        conn: &mut impl Conn,
        options: PaginationOptions,
    ) -> QueryResult<(Vec<ReverseDependency>, i64)> {
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
    pub fn index_metadata(&self, conn: &mut impl Conn) -> QueryResult<Vec<crates_io_index::Crate>> {
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

pub struct OwnerInviteEmail {
    /// The destination email address for this email.
    recipient_email_address: String,

    /// Email body variables.
    user_name: String,
    domain: String,
    crate_name: String,
    token: SecretString,
}

impl OwnerInviteEmail {
    pub fn recipient_email_address(&self) -> &str {
        &self.recipient_email_address
    }
}

impl Email for OwnerInviteEmail {
    fn subject(&self) -> String {
        format!(
            "crates.io: Ownership invitation for \"{}\"",
            self.crate_name
        )
    }

    fn body(&self) -> String {
        format!(
            "{user_name} has invited you to become an owner of the crate {crate_name}!\n
Visit https://{domain}/accept-invite/{token} to accept this invitation,
or go to https://{domain}/me/pending-invites to manage all of your crate ownership invitations.",
            user_name = self.user_name,
            domain = self.domain,
            crate_name = self.crate_name,
            token = self.token.expose_secret(),
        )
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

#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum InvalidFeature {
    #[error("feature cannot be empty")]
    Empty,
    #[error(
        "invalid character `{0}` in feature `{1}`, the first character must be \
        a Unicode XID start character or digit (most letters or `_` or `0` to \
        `9`)"
    )]
    Start(char, String),
    #[error(
        "invalid character `{0}` in feature `{1}`, characters must be Unicode \
        XID characters, `+`, `-`, or `.` (numbers, `+`, `-`, `_`, `.`, or most \
        letters)"
    )]
    Char(char, String),
    #[error(transparent)]
    DependencyName(#[from] InvalidDependencyName),
}

#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum InvalidCrateName {
    #[error("the {what} name `{name}` is too long (max {MAX_NAME_LENGTH} characters)")]
    TooLong { what: String, name: String },
    #[error("{what} name cannot be empty")]
    Empty { what: String },
    #[error(
        "the name `{name}` cannot be used as a {what} name, \
        the name cannot start with a digit"
    )]
    StartWithDigit { what: String, name: String },
    #[error(
        "invalid character `{first_char}` in {what} name: `{name}`, \
        the first character must be an ASCII character"
    )]
    Start {
        first_char: char,
        what: String,
        name: String,
    },
    #[error(
        "invalid character `{ch}` in {what} name: `{name}`, \
        characters must be an ASCII alphanumeric characters, `-`, or `_`"
    )]
    Char {
        ch: char,
        what: String,
        name: String,
    },
}

#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum InvalidDependencyName {
    #[error("the dependency name `{0}` is too long (max {MAX_NAME_LENGTH} characters)")]
    TooLong(String),
    #[error("dependency name cannot be empty")]
    Empty,
    #[error(
        "the name `{0}` cannot be used as a dependency name, \
        the name cannot start with a digit"
    )]
    StartWithDigit(String),
    #[error(
        "invalid character `{0}` in dependency name: `{1}`, \
        the first character must be an ASCII character, or `_`"
    )]
    Start(char, String),
    #[error(
        "invalid character `{0}` in dependency name: `{1}`, \
        characters must be an ASCII alphanumeric characters, `-`, or `_`"
    )]
    Char(char, String),
}

#[cfg(test)]
mod tests {
    use crate::models::Crate;

    #[test]
    fn validate_crate_name() {
        use super::{InvalidCrateName, MAX_NAME_LENGTH};

        assert_ok!(Crate::validate_crate_name("crate", "foo"));
        assert_err_eq!(
            Crate::validate_crate_name("crate", "京"),
            InvalidCrateName::Start {
                first_char: '京',
                what: "crate".into(),
                name: "京".into()
            }
        );
        assert_err_eq!(
            Crate::validate_crate_name("crate", ""),
            InvalidCrateName::Empty {
                what: "crate".into()
            }
        );
        assert_err_eq!(
            Crate::validate_crate_name("crate", "💝"),
            InvalidCrateName::Start {
                first_char: '💝',
                what: "crate".into(),
                name: "💝".into()
            }
        );
        assert_ok!(Crate::validate_crate_name("crate", "foo_underscore"));
        assert_ok!(Crate::validate_crate_name("crate", "foo-dash"));
        assert_err_eq!(
            Crate::validate_crate_name("crate", "foo+plus"),
            InvalidCrateName::Char {
                ch: '+',
                what: "crate".into(),
                name: "foo+plus".into()
            }
        );
        assert_err_eq!(
            Crate::validate_crate_name("crate", "_foo"),
            InvalidCrateName::Start {
                first_char: '_',
                what: "crate".into(),
                name: "_foo".into()
            }
        );
        assert_err_eq!(
            Crate::validate_crate_name("crate", "-foo"),
            InvalidCrateName::Start {
                first_char: '-',
                what: "crate".into(),
                name: "-foo".into()
            }
        );
        assert_err_eq!(
            Crate::validate_crate_name("crate", "123"),
            InvalidCrateName::StartWithDigit {
                what: "crate".into(),
                name: "123".into()
            }
        );
        assert_err_eq!(
            Crate::validate_crate_name("crate", "o".repeat(MAX_NAME_LENGTH + 1).as_str()),
            InvalidCrateName::TooLong {
                what: "crate".into(),
                name: "o".repeat(MAX_NAME_LENGTH + 1).as_str().into()
            }
        );
    }

    #[test]
    fn validate_dependency_name() {
        use super::{InvalidDependencyName, MAX_NAME_LENGTH};

        assert_ok!(Crate::validate_dependency_name("foo"));
        assert_err_eq!(
            Crate::validate_dependency_name("京"),
            InvalidDependencyName::Start('京', "京".into())
        );
        assert_err_eq!(
            Crate::validate_dependency_name(""),
            InvalidDependencyName::Empty
        );
        assert_err_eq!(
            Crate::validate_dependency_name("💝"),
            InvalidDependencyName::Start('💝', "💝".into())
        );
        assert_ok!(Crate::validate_dependency_name("foo_underscore"));
        assert_ok!(Crate::validate_dependency_name("foo-dash"));
        assert_err_eq!(
            Crate::validate_dependency_name("foo+plus"),
            InvalidDependencyName::Char('+', "foo+plus".into())
        );
        // Starting with an underscore is a valid dependency name.
        assert_ok!(Crate::validate_dependency_name("_foo"));
        assert_err_eq!(
            Crate::validate_dependency_name("-foo"),
            InvalidDependencyName::Start('-', "-foo".into())
        );
        assert_err_eq!(
            Crate::validate_dependency_name("o".repeat(MAX_NAME_LENGTH + 1).as_str()),
            InvalidDependencyName::TooLong("o".repeat(MAX_NAME_LENGTH + 1).as_str().into())
        );
    }

    #[test]
    fn validate_feature_names() {
        use super::InvalidDependencyName;
        use super::InvalidFeature;

        assert_ok!(Crate::validate_feature("foo"));
        assert_ok!(Crate::validate_feature("1foo"));
        assert_ok!(Crate::validate_feature("_foo"));
        assert_ok!(Crate::validate_feature("_foo-_+.1"));
        assert_ok!(Crate::validate_feature("_foo-_+.1"));
        assert_err_eq!(Crate::validate_feature(""), InvalidFeature::Empty);
        assert_err_eq!(
            Crate::validate_feature("/"),
            InvalidDependencyName::Empty.into()
        );
        assert_err_eq!(
            Crate::validate_feature("%/%"),
            InvalidDependencyName::Start('%', "%".into()).into()
        );
        assert_ok!(Crate::validate_feature("a/a"));
        assert_ok!(Crate::validate_feature("32-column-tables"));
        assert_ok!(Crate::validate_feature("c++20"));
        assert_ok!(Crate::validate_feature("krate/c++20"));
        assert_err_eq!(
            Crate::validate_feature("c++20/wow"),
            InvalidDependencyName::Char('+', "c++20".into()).into()
        );
        assert_ok!(Crate::validate_feature("foo?/bar"));
        assert_ok!(Crate::validate_feature("dep:foo"));
        assert_err_eq!(
            Crate::validate_feature("dep:foo?/bar"),
            InvalidDependencyName::Char(':', "dep:foo".into()).into()
        );
        assert_err_eq!(
            Crate::validate_feature("foo/?bar"),
            InvalidFeature::Start('?', "?bar".into())
        );
        assert_err_eq!(
            Crate::validate_feature("foo?bar"),
            InvalidFeature::Char('?', "foo?bar".into())
        );
        assert_ok!(Crate::validate_feature("bar.web"));
        assert_ok!(Crate::validate_feature("foo/bar.web"));
        assert_err_eq!(
            Crate::validate_feature("dep:0foo"),
            InvalidDependencyName::StartWithDigit("0foo".into()).into()
        );
        assert_err_eq!(
            Crate::validate_feature("0foo?/bar.web"),
            InvalidDependencyName::StartWithDigit("0foo".into()).into()
        );
    }
}
