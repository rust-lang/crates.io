use std::ascii::AsciiExt;

use chrono::{NaiveDate, NaiveDateTime};
use diesel::associations::Identifiable;
use diesel::prelude::*;
use diesel;
use license_exprs;
use semver;
use url::Url;

use app::App;
use badge::EncodableBadge;
use dependency::ReverseDependency;
use owner::{CrateOwner, Owner, OwnerKind};
use crate_owner_invitation::NewCrateOwnerInvitation;
use schema::*;
use util::{human, CargoResult};
use {Badge, Category, Keyword, User, Version};

pub mod search;
pub mod publish;
pub mod owners;
pub mod follow;
pub mod downloads;
pub mod metadata;

/// Hosts in this blacklist are known to not be hosting documentation,
/// and are possibly of malicious intent e.g. ad tracking networks, etc.
const DOCUMENTATION_BLACKLIST: [&'static str; 1] = ["rust-ci.org"];

#[derive(Debug, Insertable, Queryable, Identifiable, Associations, AsChangeset, Clone, Copy)]
#[belongs_to(Crate)]
#[primary_key(crate_id, date)]
#[table_name = "crate_downloads"]
pub struct CrateDownload {
    pub crate_id: i32,
    pub downloads: i32,
    pub date: NaiveDate,
}

#[derive(Debug, Clone, Queryable, Identifiable, Associations, AsChangeset)]
pub struct Crate {
    pub id: i32,
    pub name: String,
    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub downloads: i32,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub readme: Option<String>,
    pub license: Option<String>,
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
    crates::readme,
    crates::license,
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
    crates::readme,
    crates::license,
    crates::repository,
    crates::max_upload_size,
);

pub const MAX_NAME_LENGTH: usize = 64;

type All = diesel::dsl::Select<crates::table, AllColumns>;
type WithName<'a> = diesel::dsl::Eq<canon_crate_name<crates::name>, canon_crate_name<&'a str>>;
type ByName<'a> = diesel::dsl::Filter<All, WithName<'a>>;

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableCrate {
    pub id: String,
    pub name: String,
    pub updated_at: NaiveDateTime,
    pub versions: Option<Vec<i32>>,
    pub keywords: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub badges: Option<Vec<EncodableBadge>>,
    pub created_at: NaiveDateTime,
    pub downloads: i32,
    pub recent_downloads: Option<i64>,
    pub max_version: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    pub links: CrateLinks,
    pub exact_match: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CrateLinks {
    pub version_downloads: String,
    pub versions: Option<String>,
    pub owners: Option<String>,
    pub owner_team: Option<String>,
    pub owner_user: Option<String>,
    pub reverse_dependencies: String,
}

#[derive(Insertable, AsChangeset, Default, Debug)]
#[table_name = "crates"]
#[primary_key(name, max_upload_size)] // This is actually just to skip updating them
pub struct NewCrate<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub homepage: Option<&'a str>,
    pub documentation: Option<&'a str>,
    pub readme: Option<&'a str>,
    pub repository: Option<&'a str>,
    pub max_upload_size: Option<i32>,
    pub license: Option<&'a str>,
}

impl<'a> NewCrate<'a> {
    pub fn create_or_update(
        mut self,
        conn: &PgConnection,
        license_file: Option<&'a str>,
        uploader: i32,
    ) -> CargoResult<Crate> {
        use diesel::update;

        self.validate(license_file)?;
        self.ensure_name_not_reserved(conn)?;

        conn.transaction(|| {
            // To avoid race conditions, we try to insert
            // first so we know whether to add an owner
            if let Some(krate) = self.save_new_crate(conn, uploader)? {
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

    fn validate(&mut self, license_file: Option<&'a str>) -> CargoResult<()> {
        fn validate_url(url: Option<&str>, field: &str) -> CargoResult<()> {
            let url = match url {
                Some(s) => s,
                None => return Ok(()),
            };
            let url = Url::parse(url).map_err(|_| {
                human(&format_args!("`{}` is not a valid url: `{}`", field, url))
            })?;
            match &url.scheme()[..] {
                "http" | "https" => {}
                s => {
                    return Err(human(&format_args!(
                        "`{}` has an invalid url \
                         scheme: `{}`",
                        field,
                        s
                    )))
                }
            }
            if url.cannot_be_a_base() {
                return Err(human(&format_args!(
                    "`{}` must have relative scheme \
                     data: {}",
                    field,
                    url
                )));
            }
            Ok(())
        }

        validate_url(self.homepage, "homepage")?;
        validate_url(self.documentation, "documentation")?;
        validate_url(self.repository, "repository")?;
        self.validate_license(license_file)?;
        Ok(())
    }

    fn validate_license(&mut self, license_file: Option<&str>) -> CargoResult<()> {
        if let Some(license) = self.license {
            for part in license.split('/') {
                license_exprs::validate_license_expr(part).map_err(|e| {
                    human(&format_args!(
                        "{}; see http://opensource.org/licenses \
                         for options, and http://spdx.org/licenses/ \
                         for their identifiers",
                        e
                    ))
                })?;
            }
        } else if license_file.is_some() {
            // If no license is given, but a license file is given, flag this
            // crate as having a nonstandard license. Note that we don't
            // actually do anything else with license_file currently.
            self.license = Some("non-standard");
        }
        Ok(())
    }

    fn ensure_name_not_reserved(&self, conn: &PgConnection) -> CargoResult<()> {
        use schema::reserved_crate_names::dsl::*;
        use diesel::select;
        use diesel::dsl::exists;

        let reserved_name = select(exists(
            reserved_crate_names.filter(canon_crate_name(name).eq(canon_crate_name(self.name))),
        )).get_result::<bool>(conn)?;
        if reserved_name {
            Err(human("cannot upload a crate with a reserved name"))
        } else {
            Ok(())
        }
    }

    fn save_new_crate(&self, conn: &PgConnection, user_id: i32) -> QueryResult<Option<Crate>> {
        use schema::crates::dsl::*;

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
    pub fn with_name(name: &str) -> WithName {
        canon_crate_name(crates::name).eq(canon_crate_name(name))
    }

    pub fn by_name(name: &str) -> ByName {
        Crate::all().filter(Self::with_name(name))
    }

    pub fn all() -> All {
        crates::table.select(ALL_COLUMNS)
    }

    pub fn valid_name(name: &str) -> bool {
        let under_max_length = name.chars().take(MAX_NAME_LENGTH + 1).count() <= MAX_NAME_LENGTH;
        Crate::valid_ident(name) && under_max_length
    }

    fn valid_ident(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        name.chars().next().unwrap().is_alphabetic()
            && name.chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            && name.chars().all(|c| c.is_ascii())
    }

    pub fn valid_feature_name(name: &str) -> bool {
        let mut parts = name.split('/');
        match parts.next() {
            Some(part) if !Crate::valid_ident(part) => return false,
            None => return false,
            _ => {}
        }
        match parts.next() {
            Some(part) if !Crate::valid_ident(part) => return false,
            _ => {}
        }
        parts.next().is_none()
    }

    pub fn minimal_encodable(
        self,
        max_version: semver::Version,
        badges: Option<Vec<Badge>>,
        exact_match: bool,
        recent_downloads: Option<i64>,
    ) -> EncodableCrate {
        self.encodable(
            max_version,
            None,
            None,
            None,
            badges,
            exact_match,
            recent_downloads,
        )
    }

    #[cfg_attr(feature = "clippy", allow(too_many_arguments))]
    pub fn encodable(
        self,
        max_version: semver::Version,
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
        let badges = badges.map(|bs| bs.into_iter().map(|b| b.encodable()).collect());
        let documentation = Crate::remove_blacklisted_documentation_urls(documentation);

        EncodableCrate {
            id: name.clone(),
            name: name.clone(),
            updated_at: updated_at,
            created_at: created_at,
            downloads: downloads,
            recent_downloads: recent_downloads,
            versions: versions,
            keywords: keyword_ids,
            categories: category_ids,
            badges: badges,
            max_version: max_version.to_string(),
            documentation: documentation,
            homepage: homepage,
            exact_match: exact_match,
            description: description,
            repository: repository,
            links: CrateLinks {
                version_downloads: format!("/api/v1/crates/{}/downloads", name),
                versions: versions_link,
                owners: Some(format!("/api/v1/crates/{}/owners", name)),
                owner_team: Some(format!("/api/v1/crates/{}/owner_team", name)),
                owner_user: Some(format!("/api/v1/crates/{}/owner_user", name)),
                reverse_dependencies: format!("/api/v1/crates/{}/reverse_dependencies", name),
            },
        }
    }

    /// Return `None` if the documentation URL host matches a blacklisted host
    fn remove_blacklisted_documentation_urls(url: Option<String>) -> Option<String> {
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

        // Match documentation URL host against blacklisted host array elements
        if DOCUMENTATION_BLACKLIST.contains(&url_host) {
            None
        } else {
            Some(url)
        }
    }

    pub fn max_version(&self, conn: &PgConnection) -> CargoResult<semver::Version> {
        use schema::versions::dsl::*;

        let vs = Version::belonging_to(self)
            .select(num)
            .filter(yanked.eq(false))
            .load::<String>(conn)?
            .into_iter()
            .map(|s| semver::Version::parse(&s).unwrap());
        Ok(Version::max(vs))
    }

    pub fn owners(&self, conn: &PgConnection) -> CargoResult<Vec<Owner>> {
        let base_query = CrateOwner::belonging_to(self).filter(crate_owners::deleted.eq(false));
        let users = base_query
            .inner_join(users::table)
            .select(users::all_columns)
            .filter(crate_owners::owner_kind.eq(OwnerKind::User as i32))
            .load(conn)?
            .into_iter()
            .map(Owner::User);
        let teams = base_query
            .inner_join(teams::table)
            .select(teams::all_columns)
            .filter(crate_owners::owner_kind.eq(OwnerKind::Team as i32))
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
    ) -> CargoResult<String> {
        use diesel::insert_into;

        let owner = Owner::find_or_create_by_login(app, conn, req_user, login)?;

        match owner {
            // Users are invited and must accept before being added
            owner @ Owner::User(_) => {
                insert_into(crate_owner_invitations::table)
                    .values(&NewCrateOwnerInvitation {
                        invited_user_id: owner.id(),
                        invited_by_user_id: req_user.id,
                        crate_id: self.id,
                    })
                    .on_conflict_do_nothing()
                    .execute(conn)?;
                Ok(format!(
                    "user {} has been invited to be an owner of crate {}",
                    owner.login(),
                    self.name
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
    ) -> CargoResult<()> {
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
        offset: i64,
        limit: i64,
    ) -> QueryResult<(Vec<ReverseDependency>, i64)> {
        use diesel::dsl::sql;
        use diesel::types::{BigInt, Integer, Text};

        type SqlType = ((dependencies::SqlType, Integer, Text), BigInt);
        let rows = sql::<SqlType>(include_str!("krate_reverse_dependencies.sql"))
            .bind::<Integer, _>(self.id)
            .bind::<BigInt, _>(offset)
            .bind::<BigInt, _>(limit)
            .load::<(ReverseDependency, i64)>(conn)?;

        let (vec, counts): (_, Vec<_>) = rows.into_iter().unzip();
        let cnt = counts.into_iter().nth(0).unwrap_or(0i64);
        Ok((vec, cnt))
    }
}

#[derive(Insertable, Queryable, Identifiable, Associations, Clone, Copy, Debug)]
#[belongs_to(User)]
#[primary_key(user_id, crate_id)]
#[table_name = "follows"]
pub struct Follow {
    user_id: i32,
    crate_id: i32,
}

use diesel::types::{Date, Text};
sql_function!(canon_crate_name, canon_crate_name_t, (x: Text) -> Text);
sql_function!(to_char, to_char_t, (a: Date, b: Text) -> Text);

#[cfg(test)]
mod tests {
    use super::Crate;

    #[test]
    fn documentation_blacklist_no_url_provided() {
        assert_eq!(Crate::remove_blacklisted_documentation_urls(None), None);
    }

    #[test]
    fn documentation_blacklist_invalid_url() {
        assert_eq!(
            Crate::remove_blacklisted_documentation_urls(Some(String::from("not a url"))),
            None
        );
    }

    #[test]
    fn documentation_blacklist_url_contains_partial_match() {
        assert_eq!(
            Crate::remove_blacklisted_documentation_urls(
                Some(String::from("http://rust-ci.organists.com")),
            ),
            Some(String::from("http://rust-ci.organists.com"))
        );
    }

    #[test]
    fn documentation_blacklist_blacklisted_url() {
        assert_eq!(
            Crate::remove_blacklisted_documentation_urls(Some(String::from(
                "http://rust-ci.org/crate/crate-0.1/doc/crate-0.1",
            ),),),
            None
        );
    }
}
