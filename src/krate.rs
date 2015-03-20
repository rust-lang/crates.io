use std::ascii::AsciiExt;
use std::cmp;
use std::collections::HashMap;
use std::io::prelude::*;
use std::io;
use std::iter::repeat;
use std::mem;
use std::sync::Arc;
use std::time::Duration;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use curl::http;
use pg::types::{ToSql, Slice};
use pg;
use rustc_serialize::hex::ToHex;
use rustc_serialize::json;
use semver;
use time::Timespec;
use url::{self, Url};

use {Model, User, Keyword, Version};
use app::{App, RequestApp};
use db::{Connection, RequestTransaction};
use dependency::{Dependency, EncodableDependency};
use download::{VersionDownload, EncodableVersionDownload};
use git;
use keyword::EncodableKeyword;
use upload;
use user::{RequestUser, EncodableUser};
use util::errors::{NotFound, CargoError};
use util::{LimitErrorReader, HashingReader};
use util::{RequestUtils, CargoResult, internal, ChainError, human};
use version::EncodableVersion;

#[derive(Clone)]
pub struct Crate {
    pub id: i32,
    pub name: String,
    pub user_id: i32,
    pub updated_at: Timespec,
    pub created_at: Timespec,
    pub downloads: i32,
    pub max_version: semver::Version,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub readme: Option<String>,
    pub keywords: Vec<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct EncodableCrate {
    pub id: String,
    pub name: String,
    pub updated_at: String,
    pub versions: Option<Vec<i32>>,
    pub created_at: String,
    pub downloads: i32,
    pub max_version: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub keywords: Vec<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub links: CrateLinks,
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct CrateLinks {
    pub version_downloads: String,
    pub versions: Option<String>,
    pub owners: Option<String>,
    pub reverse_dependencies: String,
}

impl Crate {
    pub fn find(conn: &Connection, id: i32) -> CargoResult<Crate> {
        Model::find(conn, id)
    }

    pub fn find_by_name(conn: &Connection, name: &str) -> CargoResult<Crate> {
        let stmt = try!(conn.prepare("SELECT * FROM crates \
                                      WHERE canon_crate_name(name) =
                                            canon_crate_name($1) LIMIT 1"));
        let row = try!(stmt.query(&[&name as &ToSql])).into_iter().next();
        let row = try!(row.chain_error(|| NotFound));
        Ok(Model::from_row(&row))
    }

    pub fn find_or_insert(conn: &Connection, name: &str,
                          user_id: i32,
                          description: &Option<String>,
                          homepage: &Option<String>,
                          documentation: &Option<String>,
                          readme: &Option<String>,
                          keywords: &[String],
                          repository: &Option<String>,
                          license: &Option<String>,
                          license_file: &Option<String>) -> CargoResult<Crate> {
        let description = description.as_ref().map(|s| s.as_slice());
        let homepage = homepage.as_ref().map(|s| s.as_slice());
        let documentation = documentation.as_ref().map(|s| s.as_slice());
        let readme = readme.as_ref().map(|s| s.as_slice());
        let repository = repository.as_ref().map(|s| s.as_slice());
        let mut license = license.as_ref().map(|s| s.as_slice());
        let license_file = license_file.as_ref().map(|s| s.as_slice());
        let keywords = keywords.connect(",");
        try!(validate_url(homepage));
        try!(validate_url(documentation));
        try!(validate_url(repository));

        match license {
            // If a license is given, validate it to make sure it's actually a
            // valid license
            Some(..) => try!(validate_license(license)),

            // If no license is given, but a license file is given, flag this
            // crate as having a nonstandard license. Note that we don't
            // actually do anything else with license_file currently.
            None if license_file.is_some() => {
                license = Some("non-standard");
            }

            None => {}
        }

        // TODO: like with users, this is sadly racy
        let stmt = try!(conn.prepare("UPDATE crates
                                         SET documentation = $1,
                                             homepage = $2,
                                             description = $3,
                                             readme = $4,
                                             keywords = $5,
                                             license = $6,
                                             repository = $7
                                       WHERE canon_crate_name(name) =
                                             canon_crate_name($8)
                                   RETURNING *"));
        let rows = try!(stmt.query(&[&documentation, &homepage,
                                     &description, &readme, &keywords,
                                     &license, &repository,
                                     &name as &ToSql]));
        match rows.iter().next() {
            Some(row) => return Ok(Model::from_row(&row)),
            None => {}
        }

        // Blacklist the current set of crates in the rust distribution
        const RESERVED: &'static str = include_str!("reserved_crates.txt");

        if RESERVED.lines().any(|krate| name == krate) {
            return Err(human("cannot upload a crate with a reserved name"))
        }

        let stmt = try!(conn.prepare("INSERT INTO crates
                                      (name, user_id, created_at,
                                       updated_at, downloads, max_version,
                                       description, homepage, documentation,
                                       readme, keywords, repository, license)
                                      VALUES ($1, $2, $3, $3, 0, '0.0.0',
                                              $4, $5, $6, $7, $8, $9, $10)
                                      RETURNING *"));
        let now = ::now();
        let rows = try!(stmt.query(&[&name as &ToSql, &user_id, &now,
                                     &description, &homepage,
                                     &documentation, &readme, &keywords,
                                     &repository, &license]));
        let ret: Crate = Model::from_row(&try!(rows.iter().next().chain_error(|| {
            internal("no crate returned")
        })));

        try!(conn.execute("INSERT INTO crate_owners
                           (crate_id, user_id, created_at, updated_at, deleted)
                           VALUES ($1, $2, $3, $3, FALSE)",
                          &[&ret.id, &user_id, &now]));
        return Ok(ret);

        fn validate_url(url: Option<&str>) -> CargoResult<()> {
            let url = match url {
                Some(s) => s,
                None => return Ok(())
            };
            let url = match Url::parse(url) {
                Ok(url) => url,
                Err(..) => return Err(human(format!("not a valid url: {}", url)))
            };
            match url.scheme.as_slice() {
                "http" | "https" => {}
                _ => return Err(human(format!("not a valid url scheme: {}", url)))
            }
            match url.scheme_data {
                url::SchemeData::Relative(..) => {}
                url::SchemeData::NonRelative(..) => {
                    return Err(human(format!("not a valid url scheme: {}", url)))
                }
            }
            Ok(())
        }

        fn validate_license(license: Option<&str>) -> CargoResult<()> {
            use licenses::KNOWN_LICENSES;
            match license {
                Some(license) => {
                    let ok = license.split('/').all(|l| {
                        KNOWN_LICENSES.binary_search(&l.trim()).is_ok()
                    });
                    if ok {
                        Ok(())
                    } else {
                        Err(human(format!("unknown license `{}`, \
                                           see http://opensource.org/licenses \
                                           for options, and http://spdx.org/licenses/ \
                                           for their identifiers", license)))
                    }
                }
                None => Ok(()),
            }
        }
    }

    pub fn valid_name(name: &str) -> bool {
        if name.len() == 0 { return false }
        name.char_at(0).is_alphabetic() &&
            name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') &&
            name.chars().all(|c| c.is_ascii())
    }

    pub fn valid_feature_name(name: &str) -> bool {
        let mut parts = name.split('/');
        match parts.next() {
            Some(part) if !Crate::valid_name(part) => return false,
            None => return false,
            _ => {}
        }
        match parts.next() {
            Some(part) if !Crate::valid_name(part) => return false,
            _ => {}
        }
        parts.next().is_none()
    }

    pub fn encodable(self, versions: Option<Vec<i32>>) -> EncodableCrate {
        let Crate {
            name, created_at, updated_at, downloads, max_version, description,
            homepage, documentation, keywords, license, repository,
            readme: _, id: _, user_id: _,
        } = self;
        let versions_link = match versions {
            Some(..) => None,
            None => Some(format!("/api/v1/crates/{}/versions", name)),
        };
        EncodableCrate {
            id: name.clone(),
            name: name.clone(),
            updated_at: ::encode_time(updated_at),
            created_at: ::encode_time(created_at),
            downloads: downloads,
            versions: versions,
            max_version: max_version.to_string(),
            documentation: documentation,
            homepage: homepage,
            description: description,
            keywords: keywords,
            license: license,
            repository: repository,
            links: CrateLinks {
                version_downloads: format!("/api/v1/crates/{}/downloads", name),
                versions: versions_link,
                owners: Some(format!("/api/v1/crates/{}/owners", name)),
                reverse_dependencies: format!("/api/v1/crates/{}/reverse_dependencies", name)
            },
        }
    }

    pub fn versions(&self, conn: &Connection) -> CargoResult<Vec<Version>> {
        let stmt = try!(conn.prepare("SELECT * FROM versions \
                                      WHERE crate_id = $1"));
        let rows = try!(stmt.query(&[&self.id]));
        let mut ret = rows.iter().map(|r| {
            Model::from_row(&r)
        }).collect::<Vec<Version>>();
        ret.sort_by(|a, b| b.num.cmp(&a.num));
        Ok(ret)
    }

    pub fn owners(&self, conn: &Connection) -> CargoResult<Vec<User>> {
        let stmt = try!(conn.prepare("SELECT * FROM users
                                      INNER JOIN crate_owners
                                         ON crate_owners.user_id = users.id
                                      WHERE crate_owners.crate_id = $1
                                        AND crate_owners.deleted = FALSE"));
        let rows = try!(stmt.query(&[&self.id]));
        Ok(rows.iter().map(|r| Model::from_row(&r)).collect())
    }

    pub fn owner_add(&self, conn: &Connection, who: i32,
                     name: &str) -> CargoResult<()> {
        let user = try!(User::find_by_login(conn, name).map_err(|_| {
            human(format!("could not find user with login `{}`", name))
        }));
        try!(conn.execute("INSERT INTO crate_owners
                           (crate_id, user_id, created_at, updated_at,
                            created_by, deleted)
                           VALUES ($1, $2, $3, $3, $4, FALSE)",
                          &[&self.id, &user.id, &::now(), &who]));
        Ok(())
    }

    pub fn owner_remove(&self, conn: &Connection, _who: i32,
                        name: &str) -> CargoResult<()> {
        let user = try!(User::find_by_login(conn, name).map_err(|_| {
            human(format!("could not find user with login `{}`", name))
        }));
        try!(conn.execute("UPDATE crate_owners
                              SET deleted = TRUE, updated_at = $1
                            WHERE crate_id = $2 AND user_id = $3",
                          &[&::now(), &self.id, &user.id]));
        Ok(())
    }

    pub fn s3_path(&self, version: &str) -> String {
        format!("/crates/{}/{}-{}.crate", self.name, self.name, version)
    }

    pub fn add_version(&mut self, conn: &Connection, ver: &semver::Version,
                       features: &HashMap<String, Vec<String>>,
                       authors: &[String])
                       -> CargoResult<Version> {
        match try!(Version::find_by_num(conn, self.id, ver)) {
            Some(..) => {
                return Err(human(format!("crate version `{}` is already uploaded",
                                         ver)))
            }
            None => {}
        }
        let zero = semver::Version::parse("0.0.0").unwrap();
        if *ver > self.max_version || self.max_version == zero {
            self.max_version = ver.clone();
        }
        self.updated_at = ::now();
        try!(conn.execute("UPDATE crates SET updated_at = $1, max_version = $2
                           WHERE id = $3",
                          &[&self.updated_at, &self.max_version.to_string(),
                            &self.id]));
        Version::insert(conn, self.id, ver, features, authors)
    }

    pub fn keywords(&self, conn: &Connection) -> CargoResult<Vec<Keyword>> {
        let stmt = try!(conn.prepare("SELECT keywords.* FROM keywords
                                      LEFT JOIN crates_keywords
                                      ON keywords.id = crates_keywords.keyword_id
                                      WHERE crates_keywords.crate_id = $1"));
        let rows = try!(stmt.query(&[&self.id]));
        Ok(rows.iter().map(|r| Model::from_row(&r)).collect())
    }

    /// Returns (dependency, dependent crate name)
    pub fn reverse_dependencies(&self, conn: &Connection, offset: i64, limit: i64)
                                -> CargoResult<(Vec<(Dependency, String)>, i64)> {
        let select_sql = "
              FROM dependencies
              INNER JOIN versions
                ON versions.id = dependencies.version_id
              INNER JOIN crates
                ON crates.id = versions.crate_id
              WHERE dependencies.crate_id = $1
                AND versions.num = crates.max_version
        ";
        let fetch_sql = format!("SELECT DISTINCT ON (crate_name)
                                        dependencies.*,
                                        crates.name AS crate_name
                                        {}
                               ORDER BY crate_name ASC
                                 OFFSET $2
                                  LIMIT $3", select_sql);
        let count_sql = format!("SELECT COUNT(DISTINCT(crates.id)) {}",
                                select_sql);

        let stmt = try!(conn.prepare(&fetch_sql));
        let vec: Vec<_> = try!(stmt.query(&[&self.id, &offset, &limit]))
                                   .iter().map(|r| {
            (Model::from_row(&r), r.get("crate_name"))
        }).collect();
        let stmt = try!(conn.prepare(&count_sql));
        let cnt: i64 = try!(stmt.query(&[&self.id])).iter().next().unwrap().get(0);

        Ok((vec, cnt))
    }
}

impl Model for Crate {
    fn from_row(row: &pg::Row) -> Crate {
        let max: String = row.get("max_version");
        let kws: Option<String> = row.get("keywords");
        Crate {
            id: row.get("id"),
            name: row.get("name"),
            user_id: row.get("user_id"),
            updated_at: row.get("updated_at"),
            created_at: row.get("created_at"),
            downloads: row.get("downloads"),
            description: row.get("description"),
            documentation: row.get("documentation"),
            homepage: row.get("homepage"),
            readme: row.get("readme"),
            max_version: semver::Version::parse(max.as_slice()).unwrap(),
            keywords: kws.unwrap_or(String::new()).as_slice().split(',')
                         .filter(|s| !s.is_empty())
                         .map(|s| s.to_string()).collect(),
            license: row.get("license"),
            repository: row.get("repository"),
        }
    }
    fn table_name(_: Option<Crate>) -> &'static str { "crates" }
}

pub fn index(req: &mut Request) -> CargoResult<Response> {
    let conn = try!(req.tx());
    let (offset, limit) = try!(req.pagination(10, 100));
    let query = req.query();
    let sort = query.get("sort").map(|s| s.as_slice()).unwrap_or("alpha");
    let sort_sql = match sort {
        "downloads" => "ORDER BY crates.downloads DESC",
        _ => "ORDER BY crates.name ASC",
    };

    // Different queries for different parameters.
    //
    // Sure wish we had an arel-like thing here...
    let mut pattern = String::new();
    let mut id = -1;
    let (mut needs_id, mut needs_pattern) = (false, false);
    let mut args = vec![&limit as &ToSql, &offset as &ToSql];
    let (q, cnt) = query.get("q").map(|query| {
        args.insert(0, query as &ToSql);
        ("SELECT crates.* FROM crates,
                               plainto_tsquery($1) q,
                               ts_rank_cd(textsearchable_index_col, q) rank
          WHERE q @@ textsearchable_index_col
          ORDER BY rank DESC LIMIT $2 OFFSET $3".to_string(),
         "SELECT COUNT(crates.*) FROM crates,
                                      plainto_tsquery($1) q
          WHERE q @@ textsearchable_index_col".to_string())
    }).or_else(|| {
        query.get("letter").map(|letter| {
            pattern = format!("{}%", letter.as_slice().char_at(0)
                                           .to_lowercase().collect::<String>());
            needs_pattern = true;
            (format!("SELECT * FROM crates WHERE canon_crate_name(name) \
                      LIKE $1 {} LIMIT $2 OFFSET $3", sort_sql),
             "SELECT COUNT(*) FROM crates WHERE canon_crate_name(name) \
              LIKE $1".to_string())
        })
    }).or_else(|| {
        query.get("keyword").map(|kw| {
            args.insert(0, kw as &ToSql);
            let base = "FROM crates
                        INNER JOIN crates_keywords
                                ON crates.id = crates_keywords.crate_id
                        INNER JOIN keywords
                                ON crates_keywords.keyword_id = keywords.id
                        WHERE keywords.keyword = $1";
            (format!("SELECT crates.* {} {} LIMIT $2 OFFSET $3", base, sort_sql),
             format!("SELECT COUNT(crates.*) {}", base))
        })
    }).or_else(|| {
        query.get("user_id").and_then(|s| s.parse::<i32>().ok()).map(|user_id| {
            id = user_id;
            needs_id = true;
            (format!("SELECT crates.* FROM crates
                       INNER JOIN crate_owners
                          ON crate_owners.crate_id = crates.id
                       WHERE crate_owners.user_id = $1 {} \
                      LIMIT $2 OFFSET $3",
                     sort_sql),
             "SELECT COUNT(crates.*) FROM crates
               INNER JOIN crate_owners
                  ON crate_owners.crate_id = crates.id
               WHERE crate_owners.user_id = $1".to_string())
        })
    }).or_else(|| {
        query.get("following").map(|_| {
            needs_id = true;
            (format!("SELECT crates.* FROM crates
                      INNER JOIN follows
                         ON follows.crate_id = crates.id AND
                            follows.user_id = $1
                      {} LIMIT $2 OFFSET $3", sort_sql),
             "SELECT COUNT(crates.*) FROM crates
              INNER JOIN follows
                 ON follows.crate_id = crates.id AND
                    follows.user_id = $1".to_string())
        })
    }).unwrap_or_else(|| {
        (format!("SELECT * FROM crates {} LIMIT $1 OFFSET $2",
                 sort_sql),
         "SELECT COUNT(*) FROM crates".to_string())
    });

    if needs_id {
        if id == -1 {
            id = try!(req.user()).id;
        }
        args.insert(0, &id as &ToSql);
    } else if needs_pattern {
        args.insert(0, &pattern as &ToSql);
    }

    // Collect all the crates
    let stmt = try!(conn.prepare(q.as_slice()));
    let mut crates = Vec::new();
    for row in try!(stmt.query(args.as_slice())) {
        let krate: Crate = Model::from_row(&row);
        crates.push(krate.encodable(None));
    }

    // Query for the total count of crates
    let stmt = try!(conn.prepare(cnt.as_slice()));
    let args = if args.len() > 2 {&args[..1]} else {&args[..0]};
    let row = try!(stmt.query(args)).into_iter().next().unwrap();
    let total = row.get(0);

    #[derive(RustcEncodable)]
    struct R { crates: Vec<EncodableCrate>, meta: Meta }
    #[derive(RustcEncodable)]
    struct Meta { total: i64 }

    Ok(req.json(&R {
        crates: crates,
        meta: Meta { total: total },
    }))
}

pub fn summary(req: &mut Request) -> CargoResult<Response> {
    let tx = try!(req.tx());
    let num_crates = {
        let stmt = try!(tx.prepare("SELECT COUNT(*) FROM crates"));
        let rows = try!(stmt.query(&[]));
        rows.iter().next().unwrap().get("count")
    };
    let num_downloads = {
        let stmt = try!(tx.prepare("SELECT total_downloads FROM metadata"));
        let rows = try!(stmt.query(&[]));
        rows.iter().next().unwrap().get("total_downloads")
    };

    let to_crates = |stmt: pg::Statement| -> CargoResult<Vec<EncodableCrate>> {
        let rows = try!(stmt.query(&[]));
        Ok(rows.iter().map(|r| {
            let krate: Crate = Model::from_row(&r);
            krate.encodable(None)
        }).collect::<Vec<EncodableCrate>>())
    };
    let new_crates = try!(tx.prepare("SELECT * FROM crates \
                                        ORDER BY created_at DESC LIMIT 10"));
    let just_updated = try!(tx.prepare("SELECT * FROM crates \
                                        WHERE updated_at::timestamp(0) !=
                                              created_at::timestamp(0)
                                        ORDER BY updated_at DESC LIMIT 10"));
    let most_downloaded = try!(tx.prepare("SELECT * FROM crates \
                                           ORDER BY downloads DESC LIMIT 10"));

    #[derive(RustcEncodable)]
    struct R {
        num_downloads: i64,
        num_crates: i64,
        new_crates: Vec<EncodableCrate>,
        most_downloaded: Vec<EncodableCrate>,
        just_updated: Vec<EncodableCrate>,
    }
    Ok(req.json(&R {
        num_downloads: num_downloads,
        num_crates: num_crates,
        new_crates: try!(to_crates(new_crates)),
        most_downloaded: try!(to_crates(most_downloaded)),
        just_updated: try!(to_crates(just_updated)),
    }))
}

pub fn show(req: &mut Request) -> CargoResult<Response> {
    let name = &req.params()["crate_id"];
    let conn = try!(req.tx());
    let krate = try!(Crate::find_by_name(conn, name.as_slice()));
    let versions = try!(krate.versions(conn));
    let ids = versions.iter().map(|v| v.id).collect();
    let kws = try!(krate.keywords(conn));

    #[derive(RustcEncodable)]
    struct R {
        krate: EncodableCrate,
        versions: Vec<EncodableVersion>,
        keywords: Vec<EncodableKeyword>,
    }
    Ok(req.json(&R {
        krate: krate.clone().encodable(Some(ids)),
        versions: versions.into_iter().map(|v| {
            v.encodable(krate.name.as_slice())
        }).collect(),
        keywords: kws.into_iter().map(|k| k.encodable()).collect(),
    }))
}

pub fn new(req: &mut Request) -> CargoResult<Response> {
    let app = req.app().clone();

    let (new_crate, user) = try!(parse_new_headers(req));
    let name = new_crate.name.as_slice();
    let vers = &*new_crate.vers;
    let features = new_crate.features.iter().map(|(k, v)| {
        (k[..].to_string(), v.iter().map(|v| v[..].to_string()).collect())
    }).collect::<HashMap<String, Vec<String>>>();
    let keywords = new_crate.keywords.as_ref().map(|s| s.as_slice())
                                     .unwrap_or(&[]);
    let keywords = keywords.iter().map(|k| k[..].to_string()).collect::<Vec<_>>();

    // Persist the new crate, if it doesn't already exist
    let mut krate = try!(Crate::find_or_insert(try!(req.tx()), name, user.id,
                                               &new_crate.description,
                                               &new_crate.homepage,
                                               &new_crate.documentation,
                                               &new_crate.readme,
                                               keywords.as_slice(),
                                               &new_crate.repository,
                                               &new_crate.license,
                                               &new_crate.license_file));
    if krate.user_id != user.id {
        let owners = try!(krate.owners(try!(req.tx())));
        if !owners.iter().any(|o| o.id == user.id) {
            return Err(human("crate name has already been claimed by \
                              another user"))
        }
    }
    if krate.name != name {
        return Err(human(format!("crate was previously named `{}`", krate.name)))
    }

    // Persist the new version of this crate
    let mut version = try!(krate.add_version(try!(req.tx()), vers, &features,
                                             new_crate.authors.as_slice()));

    // Link this new version to all dependencies
    let mut deps = Vec::new();
    for dep in new_crate.deps.iter() {
        let (dep, krate) = try!(version.add_dependency(try!(req.tx()), dep));
        deps.push(dep.git_encode(krate.name.as_slice()));
    }

    // Update all keywords for this crate
    try!(Keyword::update_crate(try!(req.tx()), &krate, &keywords));

    // Upload the crate to S3
    let handle = http::handle();
    let mut handle = match req.app().s3_proxy {
        Some(ref proxy) => handle.proxy(&proxy[..]),
        None => handle,
    };
    let path = krate.s3_path(&vers.to_string());
    let (resp, cksum) = {
        let length = try!(read_le_u32(req.body()));
        let body = LimitErrorReader::new(req.body(), app.config.max_upload_size);
        let mut body = HashingReader::new(body);
        let resp = {
            let s3req = app.bucket.put(&mut handle, &path, &mut body,
                                       "application/x-tar")
                                  .content_length(length as usize)
                                  .header("Content-Encoding", "gzip");
            try!(s3req.exec().chain_error(|| {
                internal(format!("failed to upload to S3: `{}`", path))
            }))
        };
        (resp, body.finalize())
    };
    if resp.get_code() != 200 {
        return Err(internal(format!("failed to get a 200 response from S3: {}",
                                    resp)))
    }

    // If the git commands fail below, we shouldn't keep the crate on the
    // server.
    struct Bomb { app: Arc<App>, path: Option<String>, handle: http::Handle }
    impl Drop for Bomb {
        fn drop(&mut self) {
            match self.path {
                Some(ref path) => {
                    let _ = self.app.bucket.delete(&mut self.handle,
                                                   path.as_slice())
                                .exec();
                }
                None => {}
            }
        }
    }
    let mut bomb = Bomb { app: app.clone(), path: Some(path), handle: handle };

    // Register this crate in our local git repo.
    let git_crate = git::Crate {
        name: name.to_string(),
        vers: vers.to_string(),
        cksum: cksum.as_slice().to_hex(),
        features: features,
        deps: deps,
        yanked: Some(false),
    };
    try!(git::add_crate(&**req.app(), &git_crate).chain_error(|| {
        internal(format!("could not add crate `{}` to the git repo", name))
    }));

    // Now that we've come this far, we're committed!
    bomb.path = None;

    #[derive(RustcEncodable)]
    struct R { krate: EncodableCrate }
    Ok(req.json(&R { krate: krate.encodable(None) }))
}

fn parse_new_headers(req: &mut Request) -> CargoResult<(upload::NewCrate, User)> {
    // Make sure the tarball being uploaded looks sane
    let length = try!(req.content_length().chain_error(|| {
        human("missing header: Content-Length")
    }));
    let max = req.app().config.max_upload_size;
    if length > max as u64 {
        return Err(human(format!("max upload size is: {}", max)))
    }

    // Read the json upload request
    let amt = try!(read_le_u32(req.body())) as u64;
    if amt > max { return Err(human(format!("max upload size is: {}", max))) }
    let mut json = repeat(0).take(amt as usize).collect::<Vec<_>>();
    try!(read_fill(req.body(), &mut json));
    let json = try!(String::from_utf8(json).map_err(|_| {
        human("json body was not valid utf-8")
    }));
    let new: upload::NewCrate = try!(json::decode(&json).map_err(|e| {
        human(format!("invalid upload request: {:?}", e))
    }));

    // Make sure required fields are provided
    fn empty(s: Option<&String>) -> bool { s.map_or(true, |s| s.is_empty()) }
    let mut missing = Vec::new();

    if empty(new.description.as_ref()) {
        missing.push("description");
    }
    if empty(new.license.as_ref()) && empty(new.license_file.as_ref()) {
        missing.push("license");
    }
    if new.authors.len() == 0 || new.authors.iter().all(|s| s.is_empty()) {
        missing.push("authors");
    }
    if missing.len() > 0 {
        return Err(human(format!("missing or empty metadata fields: {}. Please \
            see http://doc.crates.io/manifest.html#package-metadata for \
            how to upload metadata", missing.connect(", "))));
    }

    let user = try!(req.user());
    Ok((new, user.clone()))
}

fn read_le_u32<R: Read + ?Sized>(r: &mut R) -> io::Result<u32> {
    let mut b = [0; 4];
    try!(read_fill(r, &mut b));
    Ok(((b[0] as u32) <<  0) |
       ((b[1] as u32) <<  8) |
       ((b[2] as u32) << 16) |
       ((b[3] as u32) << 24))
}

fn read_fill<R: Read + ?Sized>(r: &mut R, mut slice: &mut [u8])
                               -> io::Result<()> {
    while slice.len() > 0 {
        let n = try!(r.read(slice));
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "end of file reached", None))
        }
        slice = &mut mem::replace(&mut slice, &mut [])[n..];
    }
    Ok(())
}

pub fn download(req: &mut Request) -> CargoResult<Response> {
    let crate_name = req.params()["crate_id"].as_slice();
    let version = req.params()["version"].as_slice();
    let tx = try!(req.tx());
    let stmt = try!(tx.prepare("SELECT versions.id as version_id
                                FROM crates
                                INNER JOIN versions ON
                                    crates.id = versions.crate_id
                                WHERE canon_crate_name(crates.name) =
                                      canon_crate_name($1)
                                  AND versions.num = $2
                                LIMIT 1"));
    let rows = try!(stmt.query(&[&crate_name as &ToSql, &version as &ToSql]));
    let row = try!(rows.iter().next().chain_error(|| {
        human("crate or version not found")
    }));
    let version_id: i32 = row.get("version_id");
    let now = ::now();

    // Bump download counts.
    //
    // Note that this is *not* an atomic update, and that's somewhat
    // intentional. It doesn't appear that postgres supports an atomic update of
    // a counter, so we just do the hopefully "least racy" thing. This is
    // largely ok because these download counters are just that, counters. No
    // need to have super high-fidelity counter.
    //
    // Also, we only update the counter for *today*, nothing else. We have lots
    // of other counters, but they're all updated later on via the
    // update-downloads script.
    let amt = try!(tx.execute("UPDATE version_downloads
                               SET downloads = downloads + 1
                               WHERE version_id = $1 AND date($2) = date(date)",
                              &[&version_id, &now]));
    if amt == 0 {
        try!(tx.execute("INSERT INTO version_downloads
                         (version_id, downloads, counted, date, processed)
                         VALUES ($1, 1, 0, date($2), false)",
                        &[&version_id, &now]));
    }

    // Now that we've done our business, redirect to the actual data.
    let redirect_url = format!("https://{}/crates/{}/{}-{}.crate",
                               req.app().bucket.host(),
                               crate_name, crate_name, version);

    if req.wants_json() {
        #[derive(RustcEncodable)]
        struct R { url: String }
        Ok(req.json(&R{ url: redirect_url }))
    } else {
        Ok(req.redirect(redirect_url))
    }
}

pub fn downloads(req: &mut Request) -> CargoResult<Response> {
    let crate_name = req.params()["crate_id"].as_slice();
    let tx = try!(req.tx());
    let krate = try!(Crate::find_by_name(tx, crate_name));
    let mut versions = try!(krate.versions(tx));
    versions.sort_by(|a, b| b.num.cmp(&a.num));


    let to_show = &versions[..cmp::min(5, versions.len())];
    let ids = to_show.iter().map(|i| i.id).collect::<Vec<_>>();

    let cutoff_date = ::now() + Duration::days(-90);
    let stmt = try!(tx.prepare("SELECT * FROM version_downloads
                                 WHERE date > $1
                                   AND version_id = ANY($2)
                                 ORDER BY date ASC"));
    let mut downloads = Vec::new();
    for row in try!(stmt.query(&[&cutoff_date, &Slice(&ids)])) {
        let download: VersionDownload = Model::from_row(&row);
        downloads.push(download.encodable());
    }

    let stmt = try!(tx.prepare("\
          SELECT COALESCE(to_char(DATE(version_downloads.date), 'YYYY-MM-DD'), '') AS date,
                 SUM(version_downloads.downloads) AS downloads
            FROM version_downloads
           INNER JOIN versions ON
                 version_id = versions.id
           WHERE version_downloads.date > $1
             AND versions.crate_id = $2
             AND NOT (versions.id = ANY($3))
        GROUP BY DATE(version_downloads.date)
        ORDER BY DATE(version_downloads.date) ASC"));
    let mut extra = Vec::new();
    for row in try!(stmt.query(&[&cutoff_date, &krate.id, &Slice(&ids)])) {
        extra.push(ExtraDownload {
            downloads: row.get("downloads"),
            date: row.get("date")
        });
    }

    #[derive(RustcEncodable)]
    struct ExtraDownload { date: String, downloads: i64 }
    #[derive(RustcEncodable)]
    struct R { version_downloads: Vec<EncodableVersionDownload>, meta: Meta }
    #[derive(RustcEncodable)]
    struct Meta { extra_downloads: Vec<ExtraDownload> }
    let meta = Meta { extra_downloads: extra };
    Ok(req.json(&R{ version_downloads: downloads, meta: meta }))
}

fn user_and_crate(req: &mut Request) -> CargoResult<(User, Crate)> {
    let user = try!(req.user());
    let crate_name = req.params()["crate_id"].as_slice();
    let tx = try!(req.tx());
    let krate = try!(Crate::find_by_name(tx, crate_name));
    Ok((user.clone(), krate))
}

pub fn follow(req: &mut Request) -> CargoResult<Response> {
    let (user, krate) = try!(user_and_crate(req));
    let tx = try!(req.tx());
    let stmt = try!(tx.prepare("SELECT 1 FROM follows
                                WHERE user_id = $1 AND crate_id = $2"));
    let rows = try!(stmt.query(&[&user.id, &krate.id]));
    if !rows.iter().next().is_some() {
        try!(tx.execute("INSERT INTO follows (user_id, crate_id)
                         VALUES ($1, $2)", &[&user.id, &krate.id]));
    }
    #[derive(RustcEncodable)]
    struct R { ok: bool }
    Ok(req.json(&R { ok: true }))
}

pub fn unfollow(req: &mut Request) -> CargoResult<Response> {
    let (user, krate) = try!(user_and_crate(req));
    let tx = try!(req.tx());
    try!(tx.execute("DELETE FROM follows
                     WHERE user_id = $1 AND crate_id = $2",
                    &[&user.id, &krate.id]));
    #[derive(RustcEncodable)]
    struct R { ok: bool }
    Ok(req.json(&R { ok: true }))
}

pub fn following(req: &mut Request) -> CargoResult<Response> {
    let (user, krate) = try!(user_and_crate(req));
    let tx = try!(req.tx());
    let stmt = try!(tx.prepare("SELECT 1 FROM follows
                                WHERE user_id = $1 AND crate_id = $2"));
    let mut rows = try!(stmt.query(&[&user.id, &krate.id])).into_iter();
    #[derive(RustcEncodable)]
    struct R { following: bool }
    Ok(req.json(&R { following: rows.next().is_some() }))
}

pub fn versions(req: &mut Request) -> CargoResult<Response> {
    let crate_name = req.params()["crate_id"].as_slice();
    let tx = try!(req.tx());
    let krate = try!(Crate::find_by_name(tx, crate_name));
    let versions = try!(krate.versions(tx));
    let versions = versions.into_iter().map(|v| v.encodable(crate_name))
                           .collect();

    #[derive(RustcEncodable)]
    struct R { versions: Vec<EncodableVersion> }
    Ok(req.json(&R{ versions: versions }))
}

pub fn owners(req: &mut Request) -> CargoResult<Response> {
    let crate_name = req.params()["crate_id"].as_slice();
    let tx = try!(req.tx());
    let krate = try!(Crate::find_by_name(tx, crate_name));
    let owners = try!(krate.owners(tx));
    let owners = owners.into_iter().map(|u| u.encodable()).collect();

    #[derive(RustcEncodable)]
    struct R { users: Vec<EncodableUser> }
    Ok(req.json(&R{ users: owners }))
}

pub fn add_owners(req: &mut Request) -> CargoResult<Response> {
    modify_owners(req, true)
}

pub fn remove_owners(req: &mut Request) -> CargoResult<Response> {
    modify_owners(req, false)
}

fn modify_owners(req: &mut Request, add: bool) -> CargoResult<Response> {
    let mut body = String::new();
    try!(req.body().read_to_string(&mut body));
    let (user, krate) = try!(user_and_crate(req));
    let tx = try!(req.tx());
    let owners = try!(krate.owners(tx));
    if !owners.iter().any(|u| u.id == user.id) {
        return Err(human("must already be an owner to modify owners"))
    }

    #[derive(RustcDecodable)] struct Request { users: Vec<String> }
    let request: Request = try!(json::decode(&body).map_err(|_| {
        human("invalid json request")
    }));

    for login in request.users.iter() {
        if add {
            if owners.iter().any(|u| u.gh_login == *login) {
                return Err(human(format!("user `{}` is already an owner", login)))
            }
            try!(krate.owner_add(tx, user.id, login.as_slice()));
        } else {
            if login.as_slice() == user.gh_login.as_slice() {
                return Err(human("cannot remove yourself as an owner"))
            }
            try!(krate.owner_remove(tx, user.id, login.as_slice()));
        }
    }

    #[derive(RustcEncodable)]
    struct R { ok: bool }
    Ok(req.json(&R{ ok: true }))
}

pub fn reverse_dependencies(req: &mut Request) -> CargoResult<Response> {
    let name = &req.params()["crate_id"];
    let conn = try!(req.tx());
    let krate = try!(Crate::find_by_name(conn, name.as_slice()));
    let tx = try!(req.tx());
    let (offset, limit) = try!(req.pagination(10, 100));
    let (rev_deps, total) = try!(krate.reverse_dependencies(tx, offset, limit));
    let rev_deps = rev_deps.into_iter().map(|(dep, crate_name)| {
        dep.encodable(crate_name.as_slice())
    }).collect();

    #[derive(RustcEncodable)]
    struct R { dependencies: Vec<EncodableDependency>, meta: Meta }
    #[derive(RustcEncodable)]
    struct Meta { total: i64 }
    Ok(req.json(&R{ dependencies: rev_deps, meta: Meta { total: total } }))
}
