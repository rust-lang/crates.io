use std::ascii::AsciiExt;
use std::cmp;
use std::collections::HashMap;
use std::io::prelude::*;
use std::io;
use std::mem;
use std::sync::Arc;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use curl::easy::Easy;
use license_exprs;
use pg::GenericConnection;
use pg::rows::Row;
use pg::types::ToSql;
use pg;
use rustc_serialize::hex::ToHex;
use rustc_serialize::json;
use semver;
use time::{Timespec, Duration};
use url::Url;

use {Model, User, Keyword, Version, Category, Badge};
use app::{App, RequestApp};
use db::RequestTransaction;
use dependency::{Dependency, EncodableDependency};
use download::{VersionDownload, EncodableVersionDownload};
use git;
use keyword::EncodableKeyword;
use category::EncodableCategory;
use badge::EncodableBadge;
use upload;
use user::RequestUser;
use owner::{EncodableOwner, Owner, Rights, OwnerKind, Team, rights};
use util::errors::NotFound;
use util::{LimitErrorReader, HashingReader};
use util::{RequestUtils, CargoResult, internal, ChainError, human};
use version::EncodableVersion;

#[derive(Clone)]
pub struct Crate {
    pub id: i32,
    pub name: String,
    pub updated_at: Timespec,
    pub created_at: Timespec,
    pub downloads: i32,
    pub max_version: semver::Version,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub readme: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub max_upload_size: Option<i32>,
    pub max_build_info_stable: Option<semver::Version>,
    pub max_build_info_beta: Option<Timespec>,
    pub max_build_info_nightly: Option<Timespec>,
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct EncodableCrate {
    pub id: String,
    pub name: String,
    pub updated_at: String,
    pub versions: Option<Vec<i32>>,
    pub keywords: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub badges: Option<Vec<EncodableBadge>>,
    pub created_at: String,
    pub downloads: i32,
    pub max_version: String,
    pub max_build_info_stable: Option<String>,
    pub max_build_info_beta: Option<String>,
    pub max_build_info_nightly: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
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
    pub fn find_by_name(conn: &GenericConnection,
                        name: &str) -> CargoResult<Crate> {
        let stmt = try!(conn.prepare("SELECT * FROM crates \
                                      WHERE canon_crate_name(name) =
                                            canon_crate_name($1) LIMIT 1"));
        let rows = try!(stmt.query(&[&name]));
        let row = rows.iter().next();
        let row = try!(row.chain_error(|| NotFound));
        Ok(Model::from_row(&row))
    }

    pub fn find_or_insert(conn: &GenericConnection,
                          name: &str,
                          user_id: i32,
                          description: &Option<String>,
                          homepage: &Option<String>,
                          documentation: &Option<String>,
                          readme: &Option<String>,
                          repository: &Option<String>,
                          license: &Option<String>,
                          license_file: &Option<String>,
                          max_upload_size: Option<i32>)
                          -> CargoResult<Crate> {
        let description = description.as_ref().map(|s| &s[..]);
        let homepage = homepage.as_ref().map(|s| &s[..]);
        let documentation = documentation.as_ref().map(|s| &s[..]);
        let readme = readme.as_ref().map(|s| &s[..]);
        let repository = repository.as_ref().map(|s| &s[..]);
        let mut license = license.as_ref().map(|s| &s[..]);
        let license_file = license_file.as_ref().map(|s| &s[..]);
        try!(validate_url(homepage, "homepage"));
        try!(validate_url(documentation, "documentation"));
        try!(validate_url(repository, "repository"));

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
                                             license = $5,
                                             repository = $6
                                       WHERE canon_crate_name(name) =
                                             canon_crate_name($7)
                                   RETURNING *"));
        let rows = try!(stmt.query(&[&documentation, &homepage,
                                     &description, &readme,
                                     &license, &repository,
                                     &name]));
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
                                      (name, description, homepage,
                                       documentation, readme,
                                       repository, license, max_upload_size)
                                      VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                                      RETURNING *"));
        let rows = try!(stmt.query(&[&name, &description, &homepage,
                                     &documentation, &readme,
                                     &repository, &license, &max_upload_size]));
        let ret: Crate = Model::from_row(&try!(rows.iter().next().chain_error(|| {
            internal("no crate returned")
        })));

        try!(conn.execute("INSERT INTO crate_owners
                           (crate_id, owner_id, created_by, owner_kind)
                           VALUES ($1, $2, $2, $3)",
                          &[&ret.id, &user_id, &(OwnerKind::User as i32)]));
        return Ok(ret);

        fn validate_url(url: Option<&str>, field: &str) -> CargoResult<()> {
            let url = match url {
                Some(s) => s,
                None => return Ok(())
            };
            let url = try!(Url::parse(url).map_err(|_| {
                human(format!("`{}` is not a valid url: `{}`", field, url))
            }));
            match &url.scheme()[..] {
                "http" | "https" => {}
                s => return Err(human(format!("`{}` has an invalid url \
                                               scheme: `{}`", field, s)))
            }
            if url.cannot_be_a_base() {
                return Err(human(format!("`{}` must have relative scheme \
                                                        data: {}", field, url)))
            }
            Ok(())
        }

        fn validate_license(license: Option<&str>) -> CargoResult<()> {
            license.iter().flat_map(|s| s.split("/"))
                   .map(license_exprs::validate_license_expr)
                   .collect::<Result<Vec<_>, _>>()
                   .map(|_| ())
                   .map_err(|e| human(format!("{}; see http://opensource.org/licenses \
                                                  for options, and http://spdx.org/licenses/ \
                                                  for their identifiers", e)))
        }

    }

    pub fn valid_name(name: &str) -> bool {
        if name.len() == 0 { return false }
        name.chars().next().unwrap().is_alphabetic() &&
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

    pub fn minimal_encodable(self,
                             badges: Option<Vec<Badge>>) -> EncodableCrate {
        self.encodable(None, None, None, badges)
    }

    pub fn encodable(self,
                     versions: Option<Vec<i32>>,
                     keywords: Option<&[Keyword]>,
                     categories: Option<&[Category]>,
                     badges: Option<Vec<Badge>>)
                     -> EncodableCrate {
        let Crate {
            name, created_at, updated_at, downloads, max_version, description,
            homepage, documentation, license, repository,
            readme: _, id: _, max_upload_size: _,
            max_build_info_stable, max_build_info_beta, max_build_info_nightly,
        } = self;
        let versions_link = match versions {
            Some(..) => None,
            None => Some(format!("/api/v1/crates/{}/versions", name)),
        };
        let keyword_ids = keywords.map(|kws| kws.iter().map(|kw| kw.keyword.clone()).collect());
        let category_ids = categories.map(|cats| cats.iter().map(|cat| cat.slug.clone()).collect());
        let badges = badges.map(|bs| {
            bs.iter().map(|b| b.clone().encodable()).collect()
        });
        EncodableCrate {
            id: name.clone(),
            name: name.clone(),
            updated_at: ::encode_time(updated_at),
            created_at: ::encode_time(created_at),
            downloads: downloads,
            versions: versions,
            keywords: keyword_ids,
            categories: category_ids,
            badges: badges,
            max_version: max_version.to_string(),
            max_build_info_stable: max_build_info_stable.map(|s| s.to_string()),
            max_build_info_beta: max_build_info_beta.map(::encode_time),
            max_build_info_nightly: max_build_info_nightly.map(::encode_time),
            documentation: documentation,
            homepage: homepage,
            description: description,
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

    pub fn versions(&self, conn: &GenericConnection) -> CargoResult<Vec<Version>> {
        let stmt = try!(conn.prepare("SELECT * FROM versions \
                                      WHERE crate_id = $1"));
        let rows = try!(stmt.query(&[&self.id]));
        let mut ret = rows.iter().map(|r| {
            Model::from_row(&r)
        }).collect::<Vec<Version>>();
        ret.sort_by(|a, b| b.num.cmp(&a.num));
        Ok(ret)
    }

    pub fn owners(&self, conn: &GenericConnection) -> CargoResult<Vec<Owner>> {
        let stmt = try!(conn.prepare("SELECT * FROM users
                                      INNER JOIN crate_owners
                                         ON crate_owners.owner_id = users.id
                                      WHERE crate_owners.crate_id = $1
                                        AND crate_owners.deleted = FALSE
                                        AND crate_owners.owner_kind = $2"));
        let user_rows = try!(stmt.query(&[&self.id, &(OwnerKind::User as i32)]));

        let stmt = try!(conn.prepare("SELECT * FROM teams
                                      INNER JOIN crate_owners
                                         ON crate_owners.owner_id = teams.id
                                      WHERE crate_owners.crate_id = $1
                                        AND crate_owners.deleted = FALSE
                                        AND crate_owners.owner_kind = $2"));
        let team_rows = try!(stmt.query(&[&self.id, &(OwnerKind::Team as i32)]));

        let mut owners = vec![];
        owners.extend(user_rows.iter().map(|r| Owner::User(Model::from_row(&r))));
        owners.extend(team_rows.iter().map(|r| Owner::Team(Model::from_row(&r))));
        Ok(owners)
    }

    pub fn owner_add(&self, app: &App, conn: &GenericConnection, req_user: &User,
                     login: &str) -> CargoResult<()> {
        let owner = match Owner::find_by_login(conn, login) {
            Ok(owner @ Owner::User(_)) => { owner }
            Ok(Owner::Team(team)) => if try!(team.contains_user(app, req_user)) {
                Owner::Team(team)
            } else {
                return Err(human(format!("only members of {} can add it as \
                                          an owner", login)));
            },
            Err(err) => if login.contains(":") {
                Owner::Team(try!(Team::create(app, conn, login, req_user)))
            } else {
                return Err(err);
            },
        };

        // First try to un-delete if they've been soft deleted previously, then
        // do an insert if that didn't actually affect anything.
        let amt = try!(conn.execute("UPDATE crate_owners
                                        SET deleted = FALSE
                                      WHERE crate_id = $1 AND owner_id = $2
                                        AND owner_kind = $3",
                                    &[&self.id, &owner.id(), &owner.kind()]));
        assert!(amt <= 1);
        if amt == 0 {
            try!(conn.execute("INSERT INTO crate_owners
                               (crate_id, owner_id, created_by, owner_kind)
                               VALUES ($1, $2, $3, $4)",
                              &[&self.id, &owner.id(), &req_user.id,
                                &owner.kind()]));
        }

        Ok(())
    }

    pub fn owner_remove(&self,
                        conn: &GenericConnection,
                        _req_user: &User,
                        login: &str) -> CargoResult<()> {
        let owner = try!(Owner::find_by_login(conn, login).map_err(|_| {
            human(format!("could not find owner with login `{}`", login))
        }));
        try!(conn.execute("UPDATE crate_owners
                              SET deleted = TRUE
                            WHERE crate_id = $1 AND owner_id = $2
                              AND owner_kind = $3",
                          &[&self.id, &owner.id(), &owner.kind()]));
        Ok(())
    }

    pub fn s3_path(&self, version: &str) -> String {
        format!("/crates/{}/{}-{}.crate", self.name, self.name, version)
    }

    pub fn add_version(&mut self,
                       conn: &GenericConnection,
                       ver: &semver::Version,
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
            self.max_build_info_stable = None;
            self.max_build_info_beta = None;
            self.max_build_info_nightly = None;
        }

        let stmt = conn.prepare(" \
            UPDATE crates \
            SET max_version = $1,
                max_build_info_stable = $2,
                max_build_info_beta = $3,
                max_build_info_nightly = $4 \
            WHERE id = $5 \
            RETURNING updated_at")?;
        let rows = try!(stmt.query(&[
            &self.max_version.to_string(),
            &self.max_build_info_stable.as_ref().map(|vers| vers.to_string()),
            &self.max_build_info_beta,
            &self.max_build_info_nightly,
            &self.id
        ]));
        self.updated_at = rows.get(0).get("updated_at");

        Version::insert(conn, self.id, ver, features, authors)
    }

    pub fn keywords(&self, conn: &GenericConnection) -> CargoResult<Vec<Keyword>> {
        let stmt = try!(conn.prepare("SELECT keywords.* FROM keywords
                                      LEFT JOIN crates_keywords
                                      ON keywords.id = crates_keywords.keyword_id
                                      WHERE crates_keywords.crate_id = $1"));
        let rows = try!(stmt.query(&[&self.id]));
        Ok(rows.iter().map(|r| Model::from_row(&r)).collect())
    }

    pub fn categories(&self, conn: &GenericConnection) -> CargoResult<Vec<Category>> {
        let stmt = try!(conn.prepare("SELECT categories.* FROM categories \
                                      LEFT JOIN crates_categories \
                                      ON categories.id = \
                                         crates_categories.category_id \
                                      WHERE crates_categories.crate_id = $1"));
        let rows = try!(stmt.query(&[&self.id]));
        Ok(rows.iter().map(|r| Model::from_row(&r)).collect())
    }

    pub fn badges(&self, conn: &GenericConnection) -> CargoResult<Vec<Badge>> {
        let stmt = try!(conn.prepare("SELECT badges.* from badges \
                                      WHERE badges.crate_id = $1"));
        let rows = try!(stmt.query(&[&self.id]));
        Ok(rows.iter().map(|r| Model::from_row(&r)).collect())
    }

    /// Returns (dependency, dependent crate name)
    pub fn reverse_dependencies(&self,
                                conn: &GenericConnection,
                                offset: i64,
                                limit: i64)
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
    fn from_row(row: &Row) -> Crate {
        let max: String = row.get("max_version");
        let max_build_info_stable: Option<String> = row.get("max_build_info_stable");
        Crate {
            id: row.get("id"),
            name: row.get("name"),
            updated_at: row.get("updated_at"),
            created_at: row.get("created_at"),
            downloads: row.get("downloads"),
            description: row.get("description"),
            documentation: row.get("documentation"),
            homepage: row.get("homepage"),
            readme: row.get("readme"),
            max_version: semver::Version::parse(&max).unwrap(),
            license: row.get("license"),
            repository: row.get("repository"),
            max_upload_size: row.get("max_upload_size"),
            max_build_info_stable: max_build_info_stable.map(|stable| {
                semver::Version::parse(&stable).unwrap()
            }),
            max_build_info_beta: row.get("max_build_info_beta"),
            max_build_info_nightly: row.get("max_build_info_nightly"),
        }
    }
    fn table_name(_: Option<Crate>) -> &'static str { "crates" }
}

/// Handles the `GET /crates` route.
#[allow(trivial_casts)]
pub fn index(req: &mut Request) -> CargoResult<Response> {
    let conn = try!(req.tx());
    let (offset, limit) = try!(req.pagination(10, 100));
    let query = req.query();
    let sort = query.get("sort").map(|s| &s[..]).unwrap_or("alpha");
    let sort_sql = match sort {
        "downloads" => "crates.downloads DESC",
        _ => "crates.name ASC",
    };

    // Different queries for different parameters.
    //
    // Sure wish we had an arel-like thing here...
    let mut pattern = String::new();
    let mut id = -1;
    let (mut needs_id, mut needs_pattern) = (false, false);
    let mut args = vec![&limit as &ToSql, &offset];
    let (q, cnt) = query.get("q").map(|query| {
        args.insert(0, query);
        let rank_sort_sql = match sort {
            "downloads" => format!("{}, rank DESC", sort_sql),
            _ => format!("rank DESC, {}", sort_sql),
        };
        (format!("SELECT crates.* FROM crates,
                               plainto_tsquery($1) q,
                               ts_rank_cd(textsearchable_index_col, q) rank
          WHERE q @@ textsearchable_index_col
          ORDER BY name = $1 DESC, {}
          LIMIT $2 OFFSET $3", rank_sort_sql),
         "SELECT COUNT(crates.*) FROM crates,
                                      plainto_tsquery($1) q
          WHERE q @@ textsearchable_index_col".to_string())
    }).or_else(|| {
        query.get("letter").map(|letter| {
            pattern = format!("{}%", letter.chars().next().unwrap()
                                           .to_lowercase().collect::<String>());
            needs_pattern = true;
            (format!("SELECT * FROM crates WHERE canon_crate_name(name) \
                      LIKE $1 ORDER BY {} LIMIT $2 OFFSET $3", sort_sql),
             "SELECT COUNT(*) FROM crates WHERE canon_crate_name(name) \
              LIKE $1".to_string())
        })
    }).or_else(|| {
        query.get("keyword").map(|kw| {
            args.insert(0, kw);
            let base = "FROM crates
                        INNER JOIN crates_keywords
                                ON crates.id = crates_keywords.crate_id
                        INNER JOIN keywords
                                ON crates_keywords.keyword_id = keywords.id
                        WHERE lower(keywords.keyword) = lower($1)";
            (format!("SELECT crates.* {} ORDER BY {} LIMIT $2 OFFSET $3", base, sort_sql),
             format!("SELECT COUNT(crates.*) {}", base))
        })
    }).or_else(|| {
        query.get("category").map(|cat| {
            args.insert(0, cat);
            let base = "FROM crates \
                        INNER JOIN crates_categories \
                                ON crates.id = crates_categories.crate_id \
                        INNER JOIN categories \
                                ON crates_categories.category_id = \
                                   categories.id \
                        WHERE categories.slug = $1 OR \
                              categories.slug LIKE $1 || '::%'";
            (format!("SELECT crates.* {} ORDER BY {} LIMIT $2 OFFSET $3", base, sort_sql),
             format!("SELECT COUNT(crates.*) {}", base))
        })
    }).or_else(|| {
        query.get("user_id").and_then(|s| s.parse::<i32>().ok()).map(|user_id| {
            id = user_id;
            needs_id = true;
            (format!("SELECT crates.* FROM crates
                       INNER JOIN crate_owners
                          ON crate_owners.crate_id = crates.id
                       WHERE crate_owners.owner_id = $1
                       AND crate_owners.owner_kind = {}
                       ORDER BY {}
                      LIMIT $2 OFFSET $3",
                     OwnerKind::User as i32, sort_sql),
             format!("SELECT COUNT(crates.*) FROM crates
               INNER JOIN crate_owners
                  ON crate_owners.crate_id = crates.id
               WHERE crate_owners.owner_id = $1 \
                 AND crate_owners.owner_kind = {}",
                 OwnerKind::User as i32))
        })
    }).or_else(|| {
        query.get("following").map(|_| {
            needs_id = true;
            (format!("SELECT crates.* FROM crates
                      INNER JOIN follows
                         ON follows.crate_id = crates.id AND
                            follows.user_id = $1 ORDER BY
                      {} LIMIT $2 OFFSET $3", sort_sql),
             "SELECT COUNT(crates.*) FROM crates
              INNER JOIN follows
                 ON follows.crate_id = crates.id AND
                    follows.user_id = $1".to_string())
        })
    }).unwrap_or_else(|| {
        (format!("SELECT * FROM crates ORDER BY {} LIMIT $1 OFFSET $2",
                 sort_sql),
         "SELECT COUNT(*) FROM crates".to_string())
    });

    if needs_id {
        if id == -1 {
            id = try!(req.user()).id;
        }
        args.insert(0, &id);
    } else if needs_pattern {
        args.insert(0, &pattern);
    }

    // Collect all the crates
    let stmt = try!(conn.prepare(&q));
    let mut crates = Vec::new();
    for row in try!(stmt.query(&args)).iter() {
        let krate: Crate = Model::from_row(&row);
        let badges = krate.badges(conn);
        crates.push(krate.minimal_encodable(badges.ok()));
    }

    // Query for the total count of crates
    let stmt = try!(conn.prepare(&cnt));
    let args = if args.len() > 2 {&args[..1]} else {&args[..0]};
    let rows = try!(stmt.query(args));
    let row = rows.iter().next().unwrap();
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

/// Handles the `GET /summary` route.
pub fn summary(req: &mut Request) -> CargoResult<Response> {
    let tx = try!(req.tx());
    let num_crates = try!(Crate::count(tx));
    let num_downloads = {
        let stmt = try!(tx.prepare("SELECT total_downloads FROM metadata"));
        let rows = try!(stmt.query(&[]));
        rows.iter().next().unwrap().get("total_downloads")
    };

    let to_crates = |stmt: pg::stmt::Statement| -> CargoResult<Vec<_>> {
        let rows = try!(stmt.query(&[]));
        Ok(rows.iter().map(|r| {
            let krate: Crate = Model::from_row(&r);
            krate.minimal_encodable(None)
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

/// Handles the `GET /crates/:crate_id` route.
pub fn show(req: &mut Request) -> CargoResult<Response> {
    let name = &req.params()["crate_id"];
    let conn = try!(req.tx());
    let krate = try!(Crate::find_by_name(conn, &name));
    let versions = try!(krate.versions(conn));
    let ids = versions.iter().map(|v| v.id).collect();
    let kws = try!(krate.keywords(conn));
    let cats = try!(krate.categories(conn));
    let badges = try!(krate.badges(conn));

    #[derive(RustcEncodable)]
    struct R {
        krate: EncodableCrate,
        versions: Vec<EncodableVersion>,
        keywords: Vec<EncodableKeyword>,
        categories: Vec<EncodableCategory>,
    }
    Ok(req.json(&R {
        krate: krate.clone().encodable(
            Some(ids), Some(&kws), Some(&cats), Some(badges)
        ),
        versions: versions.into_iter().map(|v| {
            v.encodable(&krate.name)
        }).collect(),
        keywords: kws.into_iter().map(|k| k.encodable()).collect(),
        categories: cats.into_iter().map(|k| k.encodable()).collect(),
    }))
}

/// Handles the `PUT /crates/new` route.
pub fn new(req: &mut Request) -> CargoResult<Response> {
    let app = req.app().clone();

    let (new_crate, user) = try!(parse_new_headers(req));
    let name = &*new_crate.name;
    let vers = &*new_crate.vers;
    let features = new_crate.features.iter().map(|(k, v)| {
        (k[..].to_string(), v.iter().map(|v| v[..].to_string()).collect())
    }).collect::<HashMap<String, Vec<String>>>();
    let keywords = new_crate.keywords.as_ref().map(|s| &s[..])
                                     .unwrap_or(&[]);
    let keywords = keywords.iter().map(|k| k[..].to_string()).collect::<Vec<_>>();

    let categories = new_crate.categories.as_ref().map(|s| &s[..])
                                     .unwrap_or(&[]);
    let categories: Vec<_> = categories.iter().map(|k| k[..].to_string()).collect();

    // Persist the new crate, if it doesn't already exist
    let mut krate = try!(Crate::find_or_insert(try!(req.tx()), name, user.id,
                                               &new_crate.description,
                                               &new_crate.homepage,
                                               &new_crate.documentation,
                                               &new_crate.readme,
                                               &new_crate.repository,
                                               &new_crate.license,
                                               &new_crate.license_file,
                                               None));

    let owners = try!(krate.owners(try!(req.tx())));
    if try!(rights(req.app(), &owners, &user)) < Rights::Publish {
        return Err(human("crate name has already been claimed by \
                          another user"))
    }

    if krate.name != name {
        return Err(human(format!("crate was previously named `{}`", krate.name)))
    }

    let length = try!(req.content_length().chain_error(|| {
        human("missing header: Content-Length")
    }));
    let max = krate.max_upload_size.map(|m| m as u64)
                   .unwrap_or(app.config.max_upload_size);
    if length > max {
        return Err(human(format!("max upload size is: {}", max)))
    }

    // Persist the new version of this crate
    let mut version = try!(krate.add_version(try!(req.tx()), vers, &features,
                                             &new_crate.authors));

    // Link this new version to all dependencies
    let mut deps = Vec::new();
    for dep in new_crate.deps.iter() {
        let (dep, krate) = try!(version.add_dependency(try!(req.tx()), dep));
        deps.push(dep.git_encode(&krate.name));
    }

    // Update all keywords for this crate
    try!(Keyword::update_crate(try!(req.tx()), &krate, &keywords));

    // Update all categories for this crate, collecting any invalid categories
    // in order to be able to warn about them
    let ignored_invalid_categories = try!(
        Category::update_crate(try!(req.tx()), &krate, &categories)
    );

    // Update all badges for this crate, collecting any invalid badges in
    // order to be able to warn about them
    let ignored_invalid_badges = try!(
        Badge::update_crate(
            try!(req.tx()),
            &krate,
            new_crate.badges.unwrap_or_else(HashMap::new)
        )
    );

    // Upload the crate to S3
    let mut handle = req.app().handle();
    let path = krate.s3_path(&vers.to_string());
    let (response, cksum) = {
        let length = try!(read_le_u32(req.body()));
        let body = LimitErrorReader::new(req.body(), max);
        let mut body = HashingReader::new(body);
        let mut response = Vec::new();
        {
            let mut s3req = app.bucket.put(&mut handle, &path, &mut body,
                                           "application/x-tar",
                                           length as u64);
            s3req.write_function(|data| {
                response.extend(data);
                Ok(data.len())
            }).unwrap();
            try!(s3req.perform().chain_error(|| {
                internal(format!("failed to upload to S3: `{}`", path))
            }));
        }
        (response, body.finalize())
    };
    if handle.response_code().unwrap() != 200 {
        let response = String::from_utf8_lossy(&response);
        return Err(internal(format!("failed to get a 200 response from S3: {}",
                                    response)))
    }

    // If the git commands fail below, we shouldn't keep the crate on the
    // server.
    struct Bomb { app: Arc<App>, path: Option<String>, handle: Easy }
    impl Drop for Bomb {
        fn drop(&mut self) {
            if let Some(ref path) = self.path {
                drop(self.app.bucket.delete(&mut self.handle, &path).perform());
            }
        }
    }
    let mut bomb = Bomb { app: app.clone(), path: Some(path), handle: handle };

    // Register this crate in our local git repo.
    let git_crate = git::Crate {
        name: name.to_string(),
        vers: vers.to_string(),
        cksum: cksum.to_hex(),
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
    struct Warnings {
        invalid_categories: Vec<String>,
        invalid_badges: Vec<String>,
    }
    let warnings = Warnings {
        invalid_categories: ignored_invalid_categories,
        invalid_badges: ignored_invalid_badges,
    };

    #[derive(RustcEncodable)]
    struct R { krate: EncodableCrate, warnings: Warnings }
    Ok(req.json(&R {
        krate: krate.minimal_encodable(None),
        warnings: warnings
    }))
}

fn parse_new_headers(req: &mut Request) -> CargoResult<(upload::NewCrate, User)> {
    // Read the json upload request
    let amt = try!(read_le_u32(req.body())) as u64;
    let max = req.app().config.max_upload_size;
    if amt > max {
        return Err(human(format!("max upload size is: {}", max)))
    }
    let mut json = vec![0; amt as usize];
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
            how to upload metadata", missing.join(", "))));
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
                                      "end of file reached"))
        }
        slice = &mut mem::replace(&mut slice, &mut [])[n..];
    }
    Ok(())
}

/// Handles the `GET /crates/:crate_id/:version/download` route.
pub fn download(req: &mut Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let version = &req.params()["version"];

    // If we are a mirror, ignore failure to update download counts.
    // API-only mirrors won't have any crates in their database, and
    // incrementing the download count will look up the crate in the
    // database. Mirrors just want to pass along a redirect URL.
    if req.app().config.mirror {
        let _ = increment_download_counts(req, crate_name, version);
    } else {
        try!(increment_download_counts(req, crate_name, version));
    }

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

fn increment_download_counts(req: &Request, crate_name: &str, version: &str) -> CargoResult<()> {
    let tx = try!(req.tx());
    let stmt = try!(tx.prepare("SELECT versions.id as version_id
                                FROM crates
                                INNER JOIN versions ON
                                    crates.id = versions.crate_id
                                WHERE canon_crate_name(crates.name) =
                                      canon_crate_name($1)
                                  AND versions.num = $2
                                LIMIT 1"));
    let rows = try!(stmt.query(&[&crate_name, &version]));
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
                         (version_id) VALUES ($1)", &[&version_id]));
    }
    Ok(())
}

/// Handles the `GET /crates/:crate_id/downloads` route.
pub fn downloads(req: &mut Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
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
    for row in try!(stmt.query(&[&cutoff_date, &ids])).iter() {
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
             AND versions.id != ALL($3)
        GROUP BY DATE(version_downloads.date)
        ORDER BY DATE(version_downloads.date) ASC"));
    let mut extra = Vec::new();
    for row in try!(stmt.query(&[&cutoff_date, &krate.id, &ids])).iter() {
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
    let crate_name = &req.params()["crate_id"];
    let tx = try!(req.tx());
    let krate = try!(Crate::find_by_name(tx, crate_name));
    Ok((user.clone(), krate))
}

/// Handles the `PUT /crates/:crate_id/follow` route.
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

/// Handles the `DELETE /crates/:crate_id/follow` route.
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

/// Handles the `GET /crates/:crate_id/following` route.
pub fn following(req: &mut Request) -> CargoResult<Response> {
    let (user, krate) = try!(user_and_crate(req));
    let tx = try!(req.tx());
    let stmt = try!(tx.prepare("SELECT 1 FROM follows
                                WHERE user_id = $1 AND crate_id = $2"));
    let rows = try!(stmt.query(&[&user.id, &krate.id]));
    #[derive(RustcEncodable)]
    struct R { following: bool }
    Ok(req.json(&R { following: rows.iter().next().is_some() }))
}

/// Handles the `GET /crates/:crate_id/versions` route.
pub fn versions(req: &mut Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let tx = try!(req.tx());
    let krate = try!(Crate::find_by_name(tx, crate_name));
    let versions = try!(krate.versions(tx));
    let versions = versions.into_iter().map(|v| v.encodable(crate_name))
                           .collect();

    #[derive(RustcEncodable)]
    struct R { versions: Vec<EncodableVersion> }
    Ok(req.json(&R{ versions: versions }))
}

/// Handles the `GET /crates/:crate_id/owners` route.
pub fn owners(req: &mut Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let tx = try!(req.tx());
    let krate = try!(Crate::find_by_name(tx, crate_name));
    let owners = try!(krate.owners(tx));
    let owners = owners.into_iter().map(|o| o.encodable()).collect();

    #[derive(RustcEncodable)]
    struct R { users: Vec<EncodableOwner> }
    Ok(req.json(&R{ users: owners }))
}

/// Handles the `PUT /crates/:crate_id/owners` route.
pub fn add_owners(req: &mut Request) -> CargoResult<Response> {
    modify_owners(req, true)
}

/// Handles the `DELETE /crates/:crate_id/owners` route.
pub fn remove_owners(req: &mut Request) -> CargoResult<Response> {
    modify_owners(req, false)
}

fn modify_owners(req: &mut Request, add: bool) -> CargoResult<Response> {
    let mut body = String::new();
    try!(req.body().read_to_string(&mut body));
    let (user, krate) = try!(user_and_crate(req));
    let tx = try!(req.tx());
    let owners = try!(krate.owners(tx));

    match try!(rights(req.app(), &owners, &user)) {
        Rights::Full => {} // Yes!
        Rights::Publish => {
            return Err(human("team members don't have permission to modify owners"));
        }
        Rights::None => {
            return Err(human("only owners have permission to modify owners"));
        }
    }

    #[derive(RustcDecodable)]
    struct Request {
        // identical, for back-compat (owners preferred)
        users: Option<Vec<String>>,
        owners: Option<Vec<String>>,
    }

    let request: Request = try!(json::decode(&body).map_err(|_| {
        human("invalid json request")
    }));

    let logins = try!(request.owners.or(request.users).ok_or_else(|| {
        human("invalid json request")
    }));

    for login in &logins {
        if add {
            if owners.iter().any(|owner| owner.login() == *login) {
                return Err(human(format!("`{}` is already an owner", login)))
            }
            try!(krate.owner_add(req.app(), tx, &user, &login));
        } else {
            // Removing the team that gives you rights is prevented because
            // team members only have Rights::Publish
            if *login == user.gh_login {
                return Err(human("cannot remove yourself as an owner"))
            }
            try!(krate.owner_remove(tx, &user, &login));
        }
    }

    #[derive(RustcEncodable)]
    struct R { ok: bool }
    Ok(req.json(&R{ ok: true }))
}

/// Handles the `GET /crates/:crate_id/reverse_dependencies` route.
pub fn reverse_dependencies(req: &mut Request) -> CargoResult<Response> {
    let name = &req.params()["crate_id"];
    let conn = try!(req.tx());
    let krate = try!(Crate::find_by_name(conn, &name));
    let tx = try!(req.tx());
    let (offset, limit) = try!(req.pagination(10, 100));
    let (rev_deps, total) = try!(krate.reverse_dependencies(tx, offset, limit));
    let rev_deps = rev_deps.into_iter().map(|(dep, crate_name)| {
        dep.encodable(&crate_name)
    }).collect();

    #[derive(RustcEncodable)]
    struct R { dependencies: Vec<EncodableDependency>, meta: Meta }
    #[derive(RustcEncodable)]
    struct Meta { total: i64 }
    Ok(req.json(&R{ dependencies: rev_deps, meta: Meta { total: total } }))
}
