use std::collections::HashMap;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use diesel;
use diesel::pg::{Pg, PgConnection};
use diesel::prelude::*;
use pg::GenericConnection;
use pg::rows::Row;
use rustc_serialize::json;
use semver;
use time::{Duration, Timespec, now_utc, strptime};
use url;

use app::RequestApp;
use db::RequestTransaction;
use dependency::{Dependency, EncodableDependency};
use download::{VersionDownload, EncodableVersionDownload};
use git;
use owner::{rights, Rights};
use schema::*;
use user::RequestUser;
use util::errors::CargoError;
use util::{RequestUtils, CargoResult, ChainError, internal, human};
use {Model, Crate};
use license_exprs;

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

#[derive(Debug)]
pub struct Author {
    pub name: String,
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct EncodableVersion {
    pub id: i32,
    pub krate: String,
    pub num: String,
    pub dl_path: String,
    pub updated_at: String,
    pub created_at: String,
    pub downloads: i32,
    pub features: HashMap<String, Vec<String>>,
    pub yanked: bool,
    pub license: Option<String>,
    pub links: VersionLinks,
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct VersionLinks {
    pub dependencies: String,
    pub version_downloads: String,
    pub authors: String,
}

impl Version {
    pub fn find_by_num(
        conn: &GenericConnection,
        crate_id: i32,
        num: &semver::Version,
    ) -> CargoResult<Option<Version>> {
        let num = num.to_string();
        let stmt = conn.prepare(
            "SELECT * FROM versions \
             WHERE crate_id = $1 AND num = $2",
        )?;
        let rows = stmt.query(&[&crate_id, &num])?;
        Ok(rows.iter().next().map(|r| Model::from_row(&r)))
    }

    pub fn insert(
        conn: &GenericConnection,
        crate_id: i32,
        num: &semver::Version,
        features: &HashMap<String, Vec<String>>,
        authors: &[String],
    ) -> CargoResult<Version> {
        let num = num.to_string();
        let features = json::encode(features).unwrap();
        let stmt = conn.prepare(
            "INSERT INTO versions \
             (crate_id, num, features) \
             VALUES ($1, $2, $3) \
             RETURNING *",
        )?;
        let rows = stmt.query(&[&crate_id, &num, &features])?;
        let ret: Version = Model::from_row(&rows.iter().next().chain_error(
            || internal("no version returned"),
        )?);
        for author in authors {
            ret.add_author(conn, author)?;
        }
        Ok(ret)
    }

    pub fn valid(version: &str) -> bool {
        semver::Version::parse(version).is_ok()
    }

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

    pub fn authors(&self, conn: &GenericConnection) -> CargoResult<Vec<Author>> {
        let stmt = conn.prepare(
            "SELECT * FROM version_authors
                                       WHERE version_id = $1
                                       ORDER BY name ASC",
        )?;
        let rows = stmt.query(&[&self.id])?;
        Ok(
            rows.into_iter()
                .map(|row| Author { name: row.get("name") })
                .collect(),
        )
    }

    pub fn add_author(&self, conn: &GenericConnection, name: &str) -> CargoResult<()> {
        conn.execute(
            "INSERT INTO version_authors (version_id, name)
                           VALUES ($1, $2)",
            &[&self.id, &name],
        )?;
        Ok(())
    }

    pub fn yank(&self, conn: &GenericConnection, yanked: bool) -> CargoResult<()> {
        conn.execute(
            "UPDATE versions SET yanked = $1 WHERE id = $2",
            &[&yanked, &self.id],
        )?;
        Ok(())
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
}

impl NewVersion {
    pub fn new(
        crate_id: i32,
        num: &semver::Version,
        features: &HashMap<String, Vec<String>>,
        license: Option<String>,
        license_file: Option<&str>,
    ) -> CargoResult<Self> {
        let features = json::encode(features)?;

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
        let features = row.6.map(|s| json::decode(&s).unwrap()).unwrap_or_else(
            HashMap::new,
        );
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

impl Model for Version {
    fn from_row(row: &Row) -> Version {
        let num: String = row.get("num");
        let features: Option<String> = row.get("features");
        let features = features.map(|s| json::decode(&s).unwrap()).unwrap_or_else(
            HashMap::new,
        );
        Version {
            id: row.get("id"),
            crate_id: row.get("crate_id"),
            num: semver::Version::parse(&num).unwrap(),
            updated_at: row.get("updated_at"),
            created_at: row.get("created_at"),
            downloads: row.get("downloads"),
            features: features,
            yanked: row.get("yanked"),
            license: row.get("license"),
        }
    }
    fn table_name(_: Option<Version>) -> &'static str {
        "versions"
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

    #[derive(RustcEncodable)]
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

    #[derive(RustcEncodable)]
    struct R {
        version: EncodableVersion,
    }
    Ok(req.json(&R { version: version.encodable(&krate.name) }))
}

fn version_and_crate_old(req: &mut Request) -> CargoResult<(Version, Crate)> {
    let crate_name = &req.params()["crate_id"];
    let semver = &req.params()["version"];
    let semver = semver::Version::parse(semver).map_err(|_| {
        human(&format_args!("invalid semver: {}", semver))
    })?;
    let tx = req.tx()?;
    let krate = Crate::find_by_name(tx, crate_name)?;
    let version = Version::find_by_num(tx, krate.id, &semver)?;
    let version = version.chain_error(|| {
        human(&format_args!(
            "crate `{}` does not have a version `{}`",
            crate_name,
            semver
        ))
    })?;
    Ok((version, krate))
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

    #[derive(RustcEncodable)]
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

    #[derive(RustcEncodable)]
    struct R {
        version_downloads: Vec<EncodableVersionDownload>,
    }
    Ok(req.json(&R { version_downloads: downloads }))
}

/// Handles the `GET /crates/:crate_id/:version/authors` route.
pub fn authors(req: &mut Request) -> CargoResult<Response> {
    let (version, _) = version_and_crate_old(req)?;
    let tx = req.tx()?;
    let names = version.authors(tx)?.into_iter().map(|a| a.name).collect();

    // It was imagined that we wold associate authors with users.
    // This was never implemented. This complicated return struct
    // is all that is left, hear for backwards compatibility.
    #[derive(RustcEncodable)]
    struct R {
        users: Vec<::user::EncodableUser>,
        meta: Meta,
    }
    #[derive(RustcEncodable)]
    struct Meta {
        names: Vec<String>,
    }
    Ok(req.json(&R {
        users: vec![],
        meta: Meta { names: names },
    }))
}

/// Handles the `DELETE /crates/:crate_id/:version/yank` route.
pub fn yank(req: &mut Request) -> CargoResult<Response> {
    modify_yank(req, true)
}

/// Handles the `PUT /crates/:crate_id/:version/unyank` route.
pub fn unyank(req: &mut Request) -> CargoResult<Response> {
    modify_yank(req, false)
}

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

    #[derive(RustcEncodable)]
    struct R {
        ok: bool,
    }
    Ok(req.json(&R { ok: true }))
}
