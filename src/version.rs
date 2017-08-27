use std::collections::HashMap;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use diesel;
use diesel::pg::Pg;
use diesel::pg::upsert::*;
use diesel::prelude::*;
use semver;
use serde_json;
use time::{Duration, Timespec, now_utc, strptime};
use url;

use Crate;
use app::RequestApp;
use db::RequestTransaction;
use dependency::{Dependency, EncodableDependency};
use download::{VersionDownload, EncodableVersionDownload};
use git;
use owner::{rights, Rights};
use schema::*;
use user::RequestUser;
use util::errors::CargoError;
use util::{RequestUtils, CargoResult, human};
use license_exprs;

// This is necessary to allow joining version to both crates and readme_rendering
// in the render-readmes script.
enable_multi_table_joins!(crates, readme_rendering);

// Queryable has a custom implementation below
#[derive(Clone, Identifiable, Associations, Debug)]
#[belongs_to(Crate)]
pub struct Version {
    pub id: i32,
    pub crate_id: i32,
    pub num: semver::Version,
    pub updated_at: Timespec,
    pub created_at: Timespec,
    pub downloads: i32,
    pub features: HashMap<String, Vec<String>>,
    pub yanked: bool,
    pub license: Option<String>,
}

#[derive(Insertable, Debug)]
#[table_name = "versions"]
pub struct NewVersion {
    crate_id: i32,
    num: String,
    features: String,
    license: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableVersion {
    pub id: i32,
    #[serde(rename = "crate")]
    pub krate: String,
    pub num: String,
    pub dl_path: String,
    pub readme_path: String,
    pub updated_at: String,
    pub created_at: String,
    pub downloads: i32,
    pub features: HashMap<String, Vec<String>>,
    pub yanked: bool,
    pub license: Option<String>,
    pub links: VersionLinks,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VersionLinks {
    pub dependencies: String,
    pub version_downloads: String,
    pub authors: String,
}

#[derive(Insertable, Identifiable, Queryable, Associations, Debug, Clone, Copy)]
#[belongs_to(Version)]
#[table_name = "readme_rendering"]
#[primary_key(version_id)]
struct ReadmeRendering {
    version_id: i32,
    rendered_at: Timespec,
}

impl Version {
    pub fn encodable(self, crate_name: &str) -> EncodableVersion {
        let Version {
            id,
            num,
            updated_at,
            created_at,
            downloads,
            features,
            yanked,
            license,
            ..
        } = self;
        let num = num.to_string();
        EncodableVersion {
            dl_path: format!("/api/v1/crates/{}/{}/download", crate_name, num),
            readme_path: format!("/api/v1/crates/{}/{}/readme", crate_name, num),
            num: num.clone(),
            id: id,
            krate: crate_name.to_string(),
            updated_at: ::encode_time(updated_at),
            created_at: ::encode_time(created_at),
            downloads: downloads,
            features: features,
            yanked: yanked,
            license: license,
            links: VersionLinks {
                dependencies: format!("/api/v1/crates/{}/{}/dependencies", crate_name, num),
                version_downloads: format!("/api/v1/crates/{}/{}/downloads", crate_name, num),
                authors: format!("/api/v1/crates/{}/{}/authors", crate_name, num),
            },
        }
    }

    /// Returns (dependency, crate dependency name)
    pub fn dependencies(&self, conn: &PgConnection) -> QueryResult<Vec<(Dependency, String)>> {
        Dependency::belonging_to(self)
            .inner_join(crates::table)
            .select((dependencies::all_columns, crates::name))
            .order((dependencies::optional, crates::name))
            .load(conn)
    }

    pub fn max<T>(versions: T) -> semver::Version
    where
        T: IntoIterator<Item = semver::Version>,
    {
        versions.into_iter().max().unwrap_or_else(|| {
            semver::Version {
                major: 0,
                minor: 0,
                patch: 0,
                pre: vec![],
                build: vec![],
            }
        })
    }

    pub fn record_readme_rendering(&self, conn: &PgConnection) -> CargoResult<()> {
        let rendered = ReadmeRendering {
            version_id: self.id,
            rendered_at: ::now(),
        };

        diesel::insert(&rendered.on_conflict(
            readme_rendering::version_id,
            do_update().set(readme_rendering::rendered_at.eq(
                excluded(
                    readme_rendering::rendered_at,
                ),
            )),
        )).into(readme_rendering::table)
            .execute(&*conn)?;
        Ok(())
    }
}

impl NewVersion {
    pub fn new(
        crate_id: i32,
        num: &semver::Version,
        features: &HashMap<String, Vec<String>>,
        license: Option<String>,
        license_file: Option<&str>,
    ) -> CargoResult<Self> {
        let features = serde_json::to_string(features)?;

        let mut new_version = NewVersion {
            crate_id: crate_id,
            num: num.to_string(),
            features: features,
            license: license,
        };

        new_version.validate_license(license_file)?;

        Ok(new_version)
    }

    pub fn save(&self, conn: &PgConnection, authors: &[String]) -> CargoResult<Version> {
        use diesel::{select, insert};
        use diesel::expression::dsl::exists;
        use schema::versions::dsl::*;

        let already_uploaded = versions.filter(crate_id.eq(self.crate_id)).filter(
            num.eq(&self.num),
        );
        if select(exists(already_uploaded)).get_result(conn)? {
            return Err(human(&format_args!(
                "crate version `{}` is already \
                 uploaded",
                self.num
            )));
        }

        conn.transaction(|| {
            let version = insert(self).into(versions).get_result::<Version>(conn)?;

            let new_authors = authors
                .iter()
                .map(|s| {
                    NewAuthor {
                        version_id: version.id,
                        name: &*s,
                    }
                })
                .collect::<Vec<_>>();

            insert(&new_authors).into(version_authors::table).execute(
                conn,
            )?;
            Ok(version)
        })
    }

    fn validate_license(&mut self, license_file: Option<&str>) -> CargoResult<()> {
        if let Some(ref license) = self.license {
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
            self.license = Some(String::from("non-standard"));
        }
        Ok(())
    }
}

#[derive(Insertable, Debug)]
#[table_name = "version_authors"]
struct NewAuthor<'a> {
    version_id: i32,
    name: &'a str,
}

impl Queryable<versions::SqlType, Pg> for Version {
    type Row = (i32, i32, String, Timespec, Timespec, i32, Option<String>, bool, Option<String>);

    fn build(row: Self::Row) -> Self {
        let features = row.6
            .map(|s| serde_json::from_str(&s).unwrap())
            .unwrap_or_else(HashMap::new);
        Version {
            id: row.0,
            crate_id: row.1,
            num: semver::Version::parse(&row.2).unwrap(),
            updated_at: row.3,
            created_at: row.4,
            downloads: row.5,
            features: features,
            yanked: row.7,
            license: row.8,
        }
    }
}

/// Handles the `GET /versions` route.
// FIXME: where/how is this used?
pub fn index(req: &mut Request) -> CargoResult<Response> {
    use diesel::expression::dsl::any;
    let conn = req.db_conn()?;

    // Extract all ids requested.
    let query = url::form_urlencoded::parse(req.query_string().unwrap_or("").as_bytes());
    let ids = query
        .filter_map(|(ref a, ref b)| if *a == "ids[]" {
            b.parse().ok()
        } else {
            None
        })
        .collect::<Vec<i32>>();

    let versions = versions::table
        .inner_join(crates::table)
        .select((versions::all_columns, crates::name))
        .filter(versions::id.eq(any(ids)))
        .load::<(Version, String)>(&*conn)?
        .into_iter()
        .map(|(version, crate_name)| version.encodable(&crate_name))
        .collect();

    #[derive(Serialize)]
    struct R {
        versions: Vec<EncodableVersion>,
    }
    Ok(req.json(&R { versions: versions }))
}

/// Handles the `GET /versions/:version_id` route.
pub fn show(req: &mut Request) -> CargoResult<Response> {
    let (version, krate) = match req.params().find("crate_id") {
        Some(..) => version_and_crate(req)?,
        None => {
            let id = &req.params()["version_id"];
            let id = id.parse().unwrap_or(0);
            let conn = req.db_conn()?;
            versions::table
                .find(id)
                .inner_join(crates::table)
                .select((versions::all_columns, ::krate::ALL_COLUMNS))
                .first(&*conn)?
        }
    };

    #[derive(Serialize)]
    struct R {
        version: EncodableVersion,
    }
    Ok(req.json(&R { version: version.encodable(&krate.name) }))
}

fn version_and_crate(req: &mut Request) -> CargoResult<(Version, Crate)> {
    let crate_name = &req.params()["crate_id"];
    let semver = &req.params()["version"];
    if semver::Version::parse(semver).is_err() {
        return Err(human(&format_args!("invalid semver: {}", semver)));
    };
    let conn = req.db_conn()?;
    let krate = Crate::by_name(crate_name).first::<Crate>(&*conn)?;
    let version = Version::belonging_to(&krate)
        .filter(versions::num.eq(semver))
        .first(&*conn)
        .map_err(|_| {
            human(&format_args!(
                "crate `{}` does not have a version `{}`",
                crate_name,
                semver
            ))
        })?;
    Ok((version, krate))
}

/// Handles the `GET /crates/:crate_id/:version/dependencies` route.
pub fn dependencies(req: &mut Request) -> CargoResult<Response> {
    let (version, _) = version_and_crate(req)?;
    let conn = req.db_conn()?;
    let deps = version.dependencies(&*conn)?;
    let deps = deps.into_iter()
        .map(|(dep, crate_name)| dep.encodable(&crate_name, None))
        .collect();

    #[derive(Serialize)]
    struct R {
        dependencies: Vec<EncodableDependency>,
    }
    Ok(req.json(&R { dependencies: deps }))
}

/// Handles the `GET /crates/:crate_id/:version/downloads` route.
pub fn downloads(req: &mut Request) -> CargoResult<Response> {
    use diesel::expression::dsl::date;
    let (version, _) = version_and_crate(req)?;
    let conn = req.db_conn()?;
    let cutoff_end_date = req.query()
        .get("before_date")
        .and_then(|d| strptime(d, "%Y-%m-%d").ok())
        .unwrap_or_else(now_utc)
        .to_timespec();
    let cutoff_start_date = cutoff_end_date + Duration::days(-89);

    let downloads = VersionDownload::belonging_to(&version)
        .filter(version_downloads::date.between(
            date(cutoff_start_date)..
                date(cutoff_end_date),
        ))
        .order(version_downloads::date)
        .load(&*conn)?
        .into_iter()
        .map(VersionDownload::encodable)
        .collect();

    #[derive(Serialize)]
    struct R {
        version_downloads: Vec<EncodableVersionDownload>,
    }
    Ok(req.json(&R { version_downloads: downloads }))
}

/// Handles the `GET /crates/:crate_id/:version/authors` route.
pub fn authors(req: &mut Request) -> CargoResult<Response> {
    let (version, _) = version_and_crate(req)?;
    let conn = req.db_conn()?;
    let names = version_authors::table
        .filter(version_authors::version_id.eq(version.id))
        .select(version_authors::name)
        .order(version_authors::name)
        .load(&*conn)?;

    // It was imagined that we wold associate authors with users.
    // This was never implemented. This complicated return struct
    // is all that is left, hear for backwards compatibility.
    #[derive(Serialize)]
    struct R {
        users: Vec<::user::EncodablePublicUser>,
        meta: Meta,
    }
    #[derive(Serialize)]
    struct Meta {
        names: Vec<String>,
    }
    Ok(req.json(&R {
        users: vec![],
        meta: Meta { names: names },
    }))
}

/// Handles the `DELETE /crates/:crate_id/:version/yank` route.
/// This does not delete a crate version, it makes the crate
/// version accessible only to crates that already have a
/// `Cargo.lock` containing this version.
///
/// Notes:
/// Crate deletion is not implemented to avoid breaking builds,
/// and the goal of yanking a crate is to prevent crates
/// beginning to depend on the yanked crate version.
pub fn yank(req: &mut Request) -> CargoResult<Response> {
    modify_yank(req, true)
}

/// Handles the `PUT /crates/:crate_id/:version/unyank` route.
pub fn unyank(req: &mut Request) -> CargoResult<Response> {
    modify_yank(req, false)
}

/// Changes `yanked` flag on a crate version record
fn modify_yank(req: &mut Request, yanked: bool) -> CargoResult<Response> {
    let (version, krate) = version_and_crate(req)?;
    let user = req.user()?;
    let conn = req.db_conn()?;
    let owners = krate.owners(&conn)?;
    if rights(req.app(), &owners, user)? < Rights::Publish {
        return Err(human("must already be an owner to yank or unyank"));
    }

    if version.yanked != yanked {
        conn.transaction::<_, Box<CargoError>, _>(|| {
            diesel::update(&version)
                .set(versions::yanked.eq(yanked))
                .execute(&*conn)?;
            git::yank(&**req.app(), &krate.name, &version.num, yanked)?;
            Ok(())
        })?;
    }

    #[derive(Serialize)]
    struct R {
        ok: bool,
    }
    Ok(req.json(&R { ok: true }))
}
