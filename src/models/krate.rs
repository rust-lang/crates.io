use chrono::NaiveDateTime;
use diesel::associations::Identifiable;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::Bool;
use indexmap::IndexMap;
use url::Url;

use crate::app::App;
use crate::email;
use crate::models::version::TopVersions;
use crate::models::{
    Badge, Category, CrateOwner, CrateOwnerInvitation, Keyword, NewCrateOwnerInvitation, Owner,
    OwnerKind, ReverseDependency, User, Version,
};
use crate::util::errors::{cargo_err, AppResult};
use crate::views::{EncodableCrate, EncodableCrateLinks};

use crate::models::helpers::with_count::*;
use crate::publish_rate_limit::PublishRateLimit;
use crate::schema::*;

/// Hosts in this list are known to not be hosting documentation,
/// and are possibly of malicious intent e.g. ad tracking networks, etc.
const DOCUMENTATION_BLOCKLIST: [&str; 1] = ["rust-ci.org"];

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

type CanonCrateName<T> = self::canon_crate_name::HelperType<T>;
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

        let reserved_name = select(exists(
            reserved_crate_names.filter(canon_crate_name(name).eq(canon_crate_name(self.name))),
        ))
        .get_result::<bool>(conn)?;
        if reserved_name {
            Err(cargo_err("cannot upload a crate with a reserved name"))
        } else {
            Ok(())
        }
    }

    fn save_new_crate(&self, conn: &PgConnection, user_id: i32) -> QueryResult<Option<Crate>> {
        use crate::schema::crates::dsl::*;

        conn.transaction(|| {
            let maybe_inserted = diesel::insert_into(crates)
                .values(self)
                .on_conflict_do_nothing()
                .returning(ALL_COLUMNS)
                .get_result::<Crate>(conn)
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
            let wildcard_name = format!("%{}%", name);
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
        Self::valid_feature_name(name)
            && name
                .chars()
                .next()
                .map(char::is_alphabetic)
                .unwrap_or(false)
    }

    pub fn valid_feature_name(name: &str) -> bool {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            && name.chars().all(|c| c.is_ascii())
    }

    pub fn valid_feature(name: &str) -> bool {
        let mut parts = name.split('/');
        match parts.next() {
            Some(part) if !Crate::valid_feature_name(part) => return false,
            None => return false,
            _ => {}
        }
        match parts.next() {
            Some(part) if !Crate::valid_feature_name(part) => return false,
            _ => {}
        }
        parts.next().is_none()
    }

    pub fn minimal_encodable(
        self,
        top_versions: &TopVersions,
        badges: Option<Vec<Badge>>,
        exact_match: bool,
        recent_downloads: Option<i64>,
    ) -> EncodableCrate {
        self.encodable(
            top_versions,
            None,
            None,
            None,
            badges,
            exact_match,
            recent_downloads,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn encodable(
        self,
        top_versions: &TopVersions,
        versions: Option<Vec<i32>>,
        keywords: Option<&[Keyword]>,
        categories: Option<&[Category]>,
        badges: Option<Vec<Badge>>,
        exact_match: bool,
        recent_downloads: Option<i64>,
    ) -> EncodableCrate {
        let Crate {
            name,
            created_at,
            updated_at,
            downloads,
            description,
            homepage,
            documentation,
            repository,
            ..
        } = self;
        let versions_link = match versions {
            Some(..) => None,
            None => Some(format!("/api/v1/crates/{}/versions", name)),
        };
        let keyword_ids = keywords.map(|kws| kws.iter().map(|kw| kw.keyword.clone()).collect());
        let category_ids = categories.map(|cats| cats.iter().map(|cat| cat.slug.clone()).collect());
        let badges = badges.map(|bs| bs.into_iter().map(Badge::encodable).collect());
        let documentation = Crate::remove_blocked_documentation_urls(documentation);

        EncodableCrate {
            id: name.clone(),
            name: name.clone(),
            updated_at,
            created_at,
            downloads,
            recent_downloads,
            versions,
            keywords: keyword_ids,
            categories: category_ids,
            badges,
            max_version: top_versions.highest.to_string(),
            newest_version: top_versions.newest.to_string(),
            documentation,
            homepage,
            exact_match,
            description,
            repository,
            links: EncodableCrateLinks {
                version_downloads: format!("/api/v1/crates/{}/downloads", name),
                versions: versions_link,
                owners: Some(format!("/api/v1/crates/{}/owners", name)),
                owner_team: Some(format!("/api/v1/crates/{}/owner_team", name)),
                owner_user: Some(format!("/api/v1/crates/{}/owner_user", name)),
                reverse_dependencies: format!("/api/v1/crates/{}/reverse_dependencies", name),
            },
        }
    }

    /// Return `None` if the documentation URL host matches a blocked host
    fn remove_blocked_documentation_urls(url: Option<String>) -> Option<String> {
        // Handles if documentation URL is None
        let url = match url {
            Some(url) => url,
            None => return None,
        };

        // Handles unsuccessful parsing of documentation URL
        let parsed_url = match Url::parse(&url) {
            Ok(parsed_url) => parsed_url,
            Err(_) => return None,
        };

        // Extract host string from documentation URL
        let url_host = match parsed_url.host_str() {
            Some(url_host) => url_host,
            None => return None,
        };

        // Match documentation URL host against blocked host array elements
        if DOCUMENTATION_BLOCKLIST.contains(&url_host) {
            None
        } else {
            Some(url)
        }
    }

    /// Return both the newest (most recently updated) and
    /// highest version (in semver order) for the current crate.
    pub fn top_versions(&self, conn: &PgConnection) -> QueryResult<TopVersions> {
        use crate::schema::versions::dsl::*;

        Ok(Version::top(
            self.versions()
                .select((updated_at, num))
                .load::<(NaiveDateTime, semver::Version)>(conn)?,
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
                let maybe_inserted = insert_into(crate_owner_invitations::table)
                    .values(&NewCrateOwnerInvitation {
                        invited_user_id: user.id,
                        invited_by_user_id: req_user.id,
                        crate_id: self.id,
                    })
                    .on_conflict_do_nothing()
                    .get_result::<CrateOwnerInvitation>(conn)
                    .optional()?;

                if let Some(ownership_invitation) = maybe_inserted {
                    if let Ok(Some(email)) = user.verified_email(&conn) {
                        email::send_owner_invite_email(
                            &email.as_str(),
                            &req_user.gh_login.as_str(),
                            &self.name.as_str(),
                            &ownership_invitation.token.as_str(),
                        );
                    }
                }

                Ok(format!(
                    "user {} has been invited to be an owner of crate {}",
                    user.gh_login, self.name
                ))
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
    pub fn reverse_dependencies(
        &self,
        conn: &PgConnection,
        params: &IndexMap<String, String>,
    ) -> AppResult<(Vec<ReverseDependency>, i64)> {
        use crate::controllers::helpers::pagination::*;
        use diesel::sql_query;
        use diesel::sql_types::{BigInt, Integer};

        // FIXME: It'd be great to support this with `.paginate` directly,
        // and get cursor/id pagination for free. But Diesel doesn't currently
        // have great support for abstracting over "Is this using `Queryable`
        // or `QueryableByName` to load things?"
        let options = PaginationOptions::new(params)?;
        let offset = options.offset().unwrap_or_default();
        let rows = sql_query(include_str!("krate_reverse_dependencies.sql"))
            .bind::<Integer, _>(self.id)
            .bind::<BigInt, _>(i64::from(offset))
            .bind::<BigInt, _>(i64::from(options.per_page))
            .load::<WithCount<ReverseDependency>>(conn)?;

        Ok(rows.records_and_total())
    }
}

use diesel::sql_types::{Date, Text};
sql_function!(fn canon_crate_name(x: Text) -> Text);
sql_function!(fn to_char(a: Date, b: Text) -> Text);

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
        assert!(krate.validate().is_err());
    }

    #[test]
    fn documentation_blocked_no_url_provided() {
        assert_eq!(Crate::remove_blocked_documentation_urls(None), None);
    }

    #[test]
    fn documentation_blocked_invalid_url() {
        assert_eq!(
            Crate::remove_blocked_documentation_urls(Some(String::from("not a url"))),
            None
        );
    }

    #[test]
    fn documentation_blocked_url_contains_partial_match() {
        assert_eq!(
            Crate::remove_blocked_documentation_urls(Some(String::from(
                "http://rust-ci.organists.com"
            )),),
            Some(String::from("http://rust-ci.organists.com"))
        );
    }

    #[test]
    fn documentation_blocked_url() {
        assert_eq!(
            Crate::remove_blocked_documentation_urls(Some(String::from(
                "http://rust-ci.org/crate/crate-0.1/doc/crate-0.1",
            ),),),
            None
        );
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
