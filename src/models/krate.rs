use chrono::NaiveDateTime;
use diesel::associations::Identifiable;
use diesel::dsl;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Text};
use secrecy::SecretString;
use thiserror::Error;

use crate::controllers::helpers::pagination::*;
use crate::models::helpers::with_count::*;
use crate::models::version::TopVersions;
use crate::models::{
    CrateOwner, CrateOwnerInvitation, NewCrateOwnerInvitationOutcome, Owner, OwnerKind,
    ReverseDependency, User, Version,
};
use crate::schema::*;
use crate::sql::canon_crate_name;
use crate::util::diesel::Conn;
use crate::util::errors::{version_not_found, AppResult};
use crate::{app::App, util::errors::BoxedAppError};

use super::Team;

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

    /// Invite `login` as an owner of this crate, returning the created
    /// [`NewOwnerInvite`].
    pub fn owner_add(
        &self,
        app: &App,
        conn: &mut impl Conn,
        req_user: &User,
        login: &str,
    ) -> Result<NewOwnerInvite, OwnerAddError> {
        use diesel::insert_into;

        let owner = Owner::find_or_create_by_login(app, conn, req_user, login)?;
        match owner {
            // Users are invited and must accept before being added
            Owner::User(user) => {
                let creation_ret =
                    CrateOwnerInvitation::create(user.id, req_user.id, self.id, conn, &app.config)
                        .map_err(BoxedAppError::from)?;

                match creation_ret {
                    NewCrateOwnerInvitationOutcome::InviteCreated { plaintext_token } => {
                        Ok(NewOwnerInvite::User(user, plaintext_token))
                    }
                    NewCrateOwnerInvitationOutcome::AlreadyExists => {
                        Err(OwnerAddError::AlreadyInvited(Box::new(user)))
                    }
                }
            }
            // Teams are added as owners immediately
            Owner::Team(team) => {
                insert_into(crate_owners::table)
                    .values(&CrateOwner {
                        crate_id: self.id,
                        owner_id: team.id,
                        created_by: req_user.id,
                        owner_kind: OwnerKind::Team,
                        email_notifications: true,
                    })
                    .on_conflict(crate_owners::table.primary_key())
                    .do_update()
                    .set(crate_owners::deleted.eq(false))
                    .execute(conn)
                    .map_err(BoxedAppError::from)?;

                Ok(NewOwnerInvite::Team(team))
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

/// Error results from a [`Crate::owner_add()`] model call.
#[derive(Debug, Error)]
pub enum OwnerAddError {
    /// An opaque [`BoxedAppError`].
    #[error("{0}")] // AppError does not impl Error
    AppError(BoxedAppError),

    /// The requested invitee already has a pending invite.
    ///
    /// Note: Teams are always immediately added, so they cannot have a pending
    /// invite to cause this error.
    #[error("user already has pending invite")]
    AlreadyInvited(Box<User>),
}

/// A [`BoxedAppError`] does not impl [`std::error::Error`] so it needs a manual
/// [`From`] impl.
impl From<BoxedAppError> for OwnerAddError {
    fn from(value: BoxedAppError) -> Self {
        Self::AppError(value)
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

impl CrateVersions for [&Crate] {
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
