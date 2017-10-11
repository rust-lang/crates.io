use std::ascii::AsciiExt;
use std::cmp;

use chrono::{NaiveDate, NaiveDateTime};
use conduit::{Request, Response};
use conduit_router::RequestParams;
use diesel::associations::Identifiable;
use diesel::expression::helper_types::Eq;
use diesel::helper_types::Select;
use diesel::pg::upsert::*;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel;
use license_exprs;
use semver;
use url::Url;

use app::{App, RequestApp};
use badge::EncodableBadge;
use category::{CrateCategory, EncodableCategory};
use db::RequestTransaction;
use dependency::{EncodableDependency, ReverseDependency};
use download::{EncodableVersionDownload, VersionDownload};
use keyword::{CrateKeyword, EncodableKeyword};
use owner::{CrateOwner, Owner, OwnerKind};
use crate_owner_invitation::NewCrateOwnerInvitation;
use schema::*;
use util::{human, CargoResult, RequestUtils};
use version::EncodableVersion;
use {Badge, Category, Keyword, Replica, User, Version};

pub mod search;
pub mod publish;
pub mod owners;
pub mod follow;

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

type CrateQuery<'a> = crates::BoxedQuery<'a, Pg, <AllColumns as Expression>::SqlType>;

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

            let target = crates::table
                .filter(canon_crate_name(crates::name).eq(canon_crate_name(self.name)));
            update(target)
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
        use diesel::expression::dsl::exists;

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
        use diesel::insert;

        conn.transaction(|| {
            let maybe_inserted = insert(&self.on_conflict_do_nothing())
                .into(crates)
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
                insert(&owner).into(crate_owners::table).execute(conn)?;
            }

            Ok(maybe_inserted)
        })
    }
}

impl Crate {
    pub fn by_name(name: &str) -> CrateQuery {
        Crate::all()
            .filter(Crate::name_canonically_equals(name))
            .into_boxed()
    }

    pub fn all() -> Select<crates::table, AllColumns> {
        crates::table.select(ALL_COLUMNS)
    }

    fn name_canonically_equals(
        s: &str,
    ) -> Eq<canon_crate_name<crates::name>, canon_crate_name<&str>> {
        canon_crate_name(crates::name).eq(canon_crate_name(s))
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
        use diesel::insert;

        let owner = Owner::find_or_create_by_login(app, conn, req_user, login)?;

        match owner {
            // Users are invited and must accept before being added
            owner @ Owner::User(_) => {
                let owner_invitation = NewCrateOwnerInvitation {
                    invited_user_id: owner.id(),
                    invited_by_user_id: req_user.id,
                    crate_id: self.id,
                };

                diesel::insert(&owner_invitation.on_conflict_do_nothing())
                    .into(crate_owner_invitations::table)
                    .execute(conn)?;

                Ok(format!(
                    "user {} has been invited to be an owner of crate {}",
                    owner.login(),
                    self.name
                ))
            }
            // Teams are added as owners immediately
            owner @ Owner::Team(_) => {
                let crate_owner = CrateOwner {
                    crate_id: self.id,
                    owner_id: owner.id(),
                    created_by: req_user.id,
                    owner_kind: OwnerKind::Team as i32,
                };

                insert(&crate_owner.on_conflict(
                    crate_owners::table.primary_key(),
                    do_update().set(crate_owners::deleted.eq(false)),
                )).into(crate_owners::table)
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
        use diesel::expression::dsl::sql;
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

/// Handles the `GET /summary` route.
pub fn summary(req: &mut Request) -> CargoResult<Response> {
    use diesel::expression::{date, now, sql, DayAndMonthIntervalDsl};
    use diesel::types::{BigInt, Nullable};
    use schema::crates::dsl::*;

    let conn = req.db_conn()?;
    let num_crates = crates.count().get_result(&*conn)?;
    let num_downloads = metadata::table
        .select(metadata::total_downloads)
        .get_result(&*conn)?;

    let encode_crates = |krates: Vec<Crate>| -> CargoResult<Vec<_>> {
        Version::belonging_to(&krates)
            .filter(versions::yanked.eq(false))
            .load::<Version>(&*conn)?
            .grouped_by(&krates)
            .into_iter()
            .map(|versions| Version::max(versions.into_iter().map(|v| v.num)))
            .zip(krates)
            .map(|(max_version, krate)| {
                Ok(krate.minimal_encodable(max_version, None, false, None))
            })
            .collect()
    };

    let new_crates = crates
        .order(created_at.desc())
        .select(ALL_COLUMNS)
        .limit(10)
        .load(&*conn)?;
    let just_updated = crates
        .filter(updated_at.ne(created_at))
        .order(updated_at.desc())
        .select(ALL_COLUMNS)
        .limit(10)
        .load(&*conn)?;
    let most_downloaded = crates
        .order(downloads.desc())
        .select(ALL_COLUMNS)
        .limit(10)
        .load(&*conn)?;

    let recent_downloads = sql::<Nullable<BigInt>>("SUM(crate_downloads.downloads)");
    let most_recently_downloaded = crates
        .left_join(
            crate_downloads::table.on(
                id.eq(crate_downloads::crate_id)
                    .and(crate_downloads::date.gt(date(now - 90.days()))),
            ),
        )
        .group_by(id)
        .order(recent_downloads.desc().nulls_last())
        .limit(10)
        .select(ALL_COLUMNS)
        .load::<Crate>(&*conn)?;

    let popular_keywords = keywords::table
        .order(keywords::crates_cnt.desc())
        .limit(10)
        .load(&*conn)?
        .into_iter()
        .map(Keyword::encodable)
        .collect();

    let popular_categories = Category::toplevel(&conn, "crates", 10, 0)?
        .into_iter()
        .map(Category::encodable)
        .collect();

    #[derive(Serialize)]
    struct R {
        num_downloads: i64,
        num_crates: i64,
        new_crates: Vec<EncodableCrate>,
        most_downloaded: Vec<EncodableCrate>,
        most_recently_downloaded: Vec<EncodableCrate>,
        just_updated: Vec<EncodableCrate>,
        popular_keywords: Vec<EncodableKeyword>,
        popular_categories: Vec<EncodableCategory>,
    }
    Ok(req.json(&R {
        num_downloads: num_downloads,
        num_crates: num_crates,
        new_crates: encode_crates(new_crates)?,
        most_downloaded: encode_crates(most_downloaded)?,
        most_recently_downloaded: encode_crates(most_recently_downloaded)?,
        just_updated: encode_crates(just_updated)?,
        popular_keywords: popular_keywords,
        popular_categories: popular_categories,
    }))
}

/// Handles the `GET /crates/:crate_id` route.
pub fn show(req: &mut Request) -> CargoResult<Response> {
    use diesel::expression::dsl::*;

    let name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(name).first::<Crate>(&*conn)?;

    let mut versions = Version::belonging_to(&krate).load::<Version>(&*conn)?;
    versions.sort_by(|a, b| b.num.cmp(&a.num));
    let ids = versions.iter().map(|v| v.id).collect();

    let kws = CrateKeyword::belonging_to(&krate)
        .inner_join(keywords::table)
        .select(keywords::all_columns)
        .load(&*conn)?;
    let cats = CrateCategory::belonging_to(&krate)
        .inner_join(categories::table)
        .select(categories::all_columns)
        .load(&*conn)?;
    let recent_downloads = CrateDownload::belonging_to(&krate)
        .filter(crate_downloads::date.gt(date(now - 90.days())))
        .select(sum(crate_downloads::downloads))
        .get_result(&*conn)?;

    let badges = badges::table
        .filter(badges::crate_id.eq(krate.id))
        .load(&*conn)?;
    let max_version = krate.max_version(&conn)?;

    #[derive(Serialize)]
    struct R {
        #[serde(rename = "crate")] krate: EncodableCrate,
        versions: Vec<EncodableVersion>,
        keywords: Vec<EncodableKeyword>,
        categories: Vec<EncodableCategory>,
    }
    Ok(
        req.json(&R {
            krate: krate.clone().encodable(
                max_version,
                Some(ids),
                Some(&kws),
                Some(&cats),
                Some(badges),
                false,
                recent_downloads,
            ),
            versions: versions
                .into_iter()
                .map(|v| v.encodable(&krate.name))
                .collect(),
            keywords: kws.into_iter().map(|k| k.encodable()).collect(),
            categories: cats.into_iter().map(|k| k.encodable()).collect(),
        }),
    )
}

/// Handles the `GET /crates/:crate_id/:version/download` route.
/// This returns a URL to the location where the crate is stored.
pub fn download(req: &mut Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let version = &req.params()["version"];

    // If we are a mirror, ignore failure to update download counts.
    // API-only mirrors won't have any crates in their database, and
    // incrementing the download count will look up the crate in the
    // database. Mirrors just want to pass along a redirect URL.
    if req.app().config.mirror == Replica::ReadOnlyMirror {
        let _ = increment_download_counts(req, crate_name, version);
    } else {
        increment_download_counts(req, crate_name, version)?;
    }

    let redirect_url = req.app()
        .config
        .uploader
        .crate_location(crate_name, version)
        .ok_or_else(|| human("crate files not found"))?;

    if req.wants_json() {
        #[derive(Serialize)]
        struct R {
            url: String,
        }
        Ok(req.json(&R { url: redirect_url }))
    } else {
        Ok(req.redirect(redirect_url))
    }
}

/// Handles the `GET /crates/:crate_id/:version/readme` route.
pub fn readme(req: &mut Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let version = &req.params()["version"];

    let redirect_url = req.app()
        .config
        .uploader
        .readme_location(crate_name, version)
        .ok_or_else(|| human("crate readme not found"))?;

    if req.wants_json() {
        #[derive(Serialize)]
        struct R {
            url: String,
        }
        Ok(req.json(&R { url: redirect_url }))
    } else {
        Ok(req.redirect(redirect_url))
    }
}

fn increment_download_counts(req: &Request, crate_name: &str, version: &str) -> CargoResult<()> {
    use self::versions::dsl::*;

    let conn = req.db_conn()?;
    let version_id = versions
        .select(id)
        .filter(crate_id.eq_any(Crate::by_name(crate_name).select(crates::id)))
        .filter(num.eq(version))
        .first(&*conn)?;

    VersionDownload::create_or_increment(version_id, &conn)?;
    Ok(())
}

/// Handles the `GET /crates/:crate_id/downloads` route.
pub fn downloads(req: &mut Request) -> CargoResult<Response> {
    use diesel::expression::dsl::*;
    use diesel::types::BigInt;

    let crate_name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(crate_name).first::<Crate>(&*conn)?;

    let mut versions = Version::belonging_to(&krate).load::<Version>(&*conn)?;
    versions.sort_by(|a, b| b.num.cmp(&a.num));
    let (latest_five, rest) = versions.split_at(cmp::min(5, versions.len()));

    let downloads = VersionDownload::belonging_to(latest_five)
        .filter(version_downloads::date.gt(date(now - 90.days())))
        .order(version_downloads::date.asc())
        .load(&*conn)?
        .into_iter()
        .map(VersionDownload::encodable)
        .collect::<Vec<_>>();

    let sum_downloads = sql::<BigInt>("SUM(version_downloads.downloads)");
    let extra = VersionDownload::belonging_to(rest)
        .select((
            to_char(version_downloads::date, "YYYY-MM-DD"),
            sum_downloads,
        ))
        .filter(version_downloads::date.gt(date(now - 90.days())))
        .group_by(version_downloads::date)
        .order(version_downloads::date.asc())
        .load::<ExtraDownload>(&*conn)?;

    #[derive(Serialize, Queryable)]
    struct ExtraDownload {
        date: String,
        downloads: i64,
    }
    #[derive(Serialize)]
    struct R {
        version_downloads: Vec<EncodableVersionDownload>,
        meta: Meta,
    }
    #[derive(Serialize)]
    struct Meta {
        extra_downloads: Vec<ExtraDownload>,
    }
    let meta = Meta {
        extra_downloads: extra,
    };
    Ok(req.json(&R {
        version_downloads: downloads,
        meta: meta,
    }))
}

#[derive(Insertable, Queryable, Identifiable, Associations, Clone, Copy, Debug)]
#[belongs_to(User)]
#[primary_key(user_id, crate_id)]
#[table_name = "follows"]
pub struct Follow {
    user_id: i32,
    crate_id: i32,
}

/// Handles the `GET /crates/:crate_id/versions` route.
// FIXME: Not sure why this is necessary since /crates/:crate_id returns
// this information already, but ember is definitely requesting it
pub fn versions(req: &mut Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(crate_name).first::<Crate>(&*conn)?;
    let mut versions = Version::belonging_to(&krate).load::<Version>(&*conn)?;
    versions.sort_by(|a, b| b.num.cmp(&a.num));
    let versions = versions
        .into_iter()
        .map(|v| v.encodable(crate_name))
        .collect();

    #[derive(Serialize)]
    struct R {
        versions: Vec<EncodableVersion>,
    }
    Ok(req.json(&R { versions: versions }))
}

/// Handles the `GET /crates/:crate_id/reverse_dependencies` route.
pub fn reverse_dependencies(req: &mut Request) -> CargoResult<Response> {
    use diesel::expression::dsl::any;

    let name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(name).first::<Crate>(&*conn)?;
    let (offset, limit) = req.pagination(10, 100)?;
    let (rev_deps, total) = krate.reverse_dependencies(&*conn, offset, limit)?;
    let rev_deps: Vec<_> = rev_deps
        .into_iter()
        .map(|dep| dep.encodable(&krate.name))
        .collect();

    let version_ids: Vec<i32> = rev_deps.iter().map(|dep| dep.version_id).collect();

    let versions = versions::table
        .filter(versions::id.eq(any(version_ids)))
        .inner_join(crates::table)
        .select((versions::all_columns, crates::name))
        .load::<(Version, String)>(&*conn)?
        .into_iter()
        .map(|(version, krate_name)| version.encodable(&krate_name))
        .collect();

    #[derive(Serialize)]
    struct R {
        dependencies: Vec<EncodableDependency>,
        versions: Vec<EncodableVersion>,
        meta: Meta,
    }
    #[derive(Serialize)]
    struct Meta {
        total: i64,
    }
    Ok(req.json(&R {
        dependencies: rev_deps,
        versions,
        meta: Meta { total: total },
    }))
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
