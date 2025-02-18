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
use thiserror::Error;
use tracing::instrument;

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

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = crates, check_for_backend(diesel::pg::Pg))]
pub struct CrateName {
    pub name: String,
}

#[derive(Debug, Clone, Queryable, Identifiable, AsChangeset, QueryableByName, Selectable)]
#[diesel(table_name = crates, check_for_backend(diesel::pg::Pg))]
pub struct Crate {
    pub id: i32,
    pub name: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    max_upload_size: Option<i32>,
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
    use claims::{assert_err_eq, assert_ok};

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
