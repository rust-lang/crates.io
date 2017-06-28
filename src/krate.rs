use std::ascii::AsciiExt;
use std::cmp;
use std::collections::HashMap;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use diesel::associations::Identifiable;
use diesel::helper_types::Select;
use diesel::pg::upsert::*;
use diesel::pg::{Pg, PgConnection};
use diesel::prelude::*;
use diesel;
use diesel_full_text_search::*;
use license_exprs;
use pg::GenericConnection;
use pg::rows::Row;
use rustc_serialize::hex::ToHex;
use rustc_serialize::json;
use semver;
use time::Timespec;
use url::Url;

use app::{App, RequestApp};
use badge::EncodableBadge;
use category::{EncodableCategory, CrateCategory};
use db::RequestTransaction;
use dependency::{self, ReverseDependency, EncodableDependency};
use download::{VersionDownload, EncodableVersionDownload};
use git;
use keyword::{EncodableKeyword, CrateKeyword};
use owner::{EncodableOwner, Owner, Rights, OwnerKind, Team, rights, CrateOwner};
use schema::*;
use upload;
use user::RequestUser;
use util::errors::NotFound;
use util::{read_le_u32, read_fill};
use util::{RequestUtils, CargoResult, internal, ChainError, human};
use version::{EncodableVersion, NewVersion};
use {Model, User, Keyword, Version, Category, Badge, Replica};

#[derive(Debug, Clone, Queryable, Identifiable, AsChangeset)]
pub struct Crate {
    pub id: i32,
    pub name: String,
    pub updated_at: Timespec,
    pub created_at: Timespec,
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
type AllColumns = (crates::id,
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
                   crates::max_upload_size);

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

#[derive(RustcEncodable, RustcDecodable, Debug)]
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
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub links: CrateLinks,
    pub exact_match: bool,
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
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

            let target = crates::table.filter(canon_crate_name(crates::name).eq(
                canon_crate_name(self.name),
            ));
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
            reserved_crate_names.filter(canon_crate_name(name).eq(
                canon_crate_name(
                    self.name,
                ),
            )),
        )).get_result::<bool>(conn)?;
        if reserved_name {
            Err(human("cannot upload a crate with a reserved name"))
        } else {
            Ok(())
        }
    }

    fn save_new_crate(&self, conn: &PgConnection, user_id: i32) -> CargoResult<Option<Crate>> {
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
            .filter(canon_crate_name(crates::name).eq(canon_crate_name(name)))
            .into_boxed()
    }

    pub fn all() -> Select<crates::table, AllColumns> {
        crates::table.select(ALL_COLUMNS)
    }

    pub fn find_by_name(conn: &GenericConnection, name: &str) -> CargoResult<Crate> {
        let stmt = conn.prepare(
            "SELECT * FROM crates \
                                      WHERE canon_crate_name(name) =
                                            canon_crate_name($1) LIMIT 1",
        )?;
        let rows = stmt.query(&[&name])?;
        let row = rows.iter().next();
        let row = row.chain_error(|| NotFound)?;
        Ok(Model::from_row(&row))
    }

    // This is cleaned up by the diesel port
    #[cfg_attr(feature = "lint", allow(too_many_arguments))]
    pub fn find_or_insert(
        conn: &GenericConnection,
        name: &str,
        user_id: i32,
        description: &Option<String>,
        homepage: &Option<String>,
        documentation: &Option<String>,
        readme: &Option<String>,
        repository: &Option<String>,
        license: &Option<String>,
        license_file: &Option<String>,
        max_upload_size: Option<i32>,
    ) -> CargoResult<Crate> {
        let description = description.as_ref().map(|s| &s[..]);
        let homepage = homepage.as_ref().map(|s| &s[..]);
        let documentation = documentation.as_ref().map(|s| &s[..]);
        let readme = readme.as_ref().map(|s| &s[..]);
        let repository = repository.as_ref().map(|s| &s[..]);
        let mut license = license.as_ref().map(|s| &s[..]);
        let license_file = license_file.as_ref().map(|s| &s[..]);
        validate_url(homepage, "homepage")?;
        validate_url(documentation, "documentation")?;
        validate_url(repository, "repository")?;

        match license {
            // If a license is given, validate it to make sure it's actually a
            // valid license
            Some(..) => validate_license(license)?,

            // If no license is given, but a license file is given, flag this
            // crate as having a nonstandard license. Note that we don't
            // actually do anything else with license_file currently.
            None if license_file.is_some() => {
                license = Some("non-standard");
            }

            None => {}
        }

        // TODO: like with users, this is sadly racy
        let stmt = conn.prepare(
            "UPDATE crates
                                         SET documentation = $1,
                                             homepage = $2,
                                             description = $3,
                                             readme = $4,
                                             license = $5,
                                             repository = $6
                                       WHERE canon_crate_name(name) =
                                             canon_crate_name($7)
                                   RETURNING *",
        )?;
        let rows = stmt.query(
            &[
                &documentation,
                &homepage,
                &description,
                &readme,
                &license,
                &repository,
                &name,
            ],
        )?;
        if let Some(row) = rows.iter().next() {
            return Ok(Model::from_row(&row));
        }

        let stmt = conn.prepare(
            "SELECT 1 FROM reserved_crate_names
                                 WHERE canon_crate_name(name) =
                                       canon_crate_name($1)",
        )?;
        let rows = stmt.query(&[&name])?;
        if !rows.is_empty() {
            return Err(human("cannot upload a crate with a reserved name"));
        }

        let stmt = conn.prepare(
            "INSERT INTO crates
                                      (name, description, homepage,
                                       documentation, readme,
                                       repository, license, max_upload_size)
                                      VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                                      RETURNING *",
        )?;
        let rows = stmt.query(
            &[
                &name,
                &description,
                &homepage,
                &documentation,
                &readme,
                &repository,
                &license,
                &max_upload_size,
            ],
        )?;
        let ret: Crate = Model::from_row(&rows.iter().next().chain_error(
            || internal("no crate returned"),
        )?);

        conn.execute(
            "INSERT INTO crate_owners
                           (crate_id, owner_id, created_by, owner_kind)
                           VALUES ($1, $2, $2, $3)",
            &[&ret.id, &user_id, &(OwnerKind::User as i32)],
        )?;
        return Ok(ret);

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

        fn validate_license(license: Option<&str>) -> CargoResult<()> {
            license
                .iter()
                .flat_map(|s| s.split('/'))
                .map(license_exprs::validate_license_expr)
                .collect::<Result<Vec<_>, _>>()
                .map(|_| ())
                .map_err(|e| {
                    human(&format_args!(
                        "{}; see http://opensource.org/licenses \
                         for options, and http://spdx.org/licenses/ \
                         for their identifiers",
                        e
                    ))
                })
        }

    }

    pub fn valid_name(name: &str) -> bool {
        let under_max_length = name.chars().take(MAX_NAME_LENGTH + 1).count() <= MAX_NAME_LENGTH;
        Crate::valid_ident(name) && under_max_length
    }

    fn valid_ident(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        name.chars().next().unwrap().is_alphabetic() &&
            name.chars().all(
                |c| c.is_alphanumeric() || c == '_' || c == '-',
            ) && name.chars().all(|c| c.is_ascii())
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
    ) -> EncodableCrate {
        self.encodable(max_version, None, None, None, badges, exact_match)
    }

    pub fn encodable(
        self,
        max_version: semver::Version,
        versions: Option<Vec<i32>>,
        keywords: Option<&[Keyword]>,
        categories: Option<&[Category]>,
        badges: Option<Vec<Badge>>,
        exact_match: bool,
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
            license,
            ..
        } = self;
        let versions_link = match versions {
            Some(..) => None,
            None => Some(format!("/api/v1/crates/{}/versions", name)),
        };
        let keyword_ids = keywords.map(|kws| kws.iter().map(|kw| kw.keyword.clone()).collect());
        let category_ids = categories.map(|cats| cats.iter().map(|cat| cat.slug.clone()).collect());
        let badges = badges.map(|bs| bs.into_iter().map(|b| b.encodable()).collect());
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
            documentation: documentation,
            homepage: homepage,
            exact_match: exact_match,
            description: description,
            repository: repository,
            license: license,
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

    pub fn max_version_old(&self, conn: &GenericConnection) -> CargoResult<semver::Version> {
        let stmt = conn.prepare(
            "SELECT num FROM versions WHERE crate_id = $1
                                 AND yanked = 'f'",
        )?;
        let rows = stmt.query(&[&self.id])?;
        Ok(Version::max(
            rows.iter().map(|r| r.get::<_, String>("num")).map(|s| {
                semver::Version::parse(&s).unwrap()
            }),
        ))
    }

    pub fn versions(&self, conn: &GenericConnection) -> CargoResult<Vec<Version>> {
        let stmt = conn.prepare(
            "SELECT * FROM versions \
             WHERE crate_id = $1",
        )?;
        let rows = stmt.query(&[&self.id])?;
        let mut ret = rows.iter()
            .map(|r| Model::from_row(&r))
            .collect::<Vec<Version>>();
        ret.sort_by(|a, b| b.num.cmp(&a.num));
        Ok(ret)
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

    pub fn owners_old(&self, conn: &GenericConnection) -> CargoResult<Vec<Owner>> {
        let stmt = conn.prepare(
            "SELECT * FROM users
                                      INNER JOIN crate_owners
                                         ON crate_owners.owner_id = users.id
                                      WHERE crate_owners.crate_id = $1
                                        AND crate_owners.deleted = FALSE
                                        AND crate_owners.owner_kind = $2",
        )?;
        let user_rows = stmt.query(&[&self.id, &(OwnerKind::User as i32)])?;

        let stmt = conn.prepare(
            "SELECT * FROM teams
                                      INNER JOIN crate_owners
                                         ON crate_owners.owner_id = teams.id
                                      WHERE crate_owners.crate_id = $1
                                        AND crate_owners.deleted = FALSE
                                        AND crate_owners.owner_kind = $2",
        )?;
        let team_rows = stmt.query(&[&self.id, &(OwnerKind::Team as i32)])?;

        let mut owners = vec![];
        owners.extend(user_rows.iter().map(|r| Owner::User(Model::from_row(&r))));
        owners.extend(team_rows.iter().map(|r| Owner::Team(Model::from_row(&r))));
        Ok(owners)
    }

    pub fn owner_add(
        &self,
        app: &App,
        conn: &PgConnection,
        req_user: &User,
        login: &str,
    ) -> CargoResult<()> {
        let owner = match Owner::find_by_login(conn, login) {
            Ok(owner @ Owner::User(_)) => owner,
            Ok(Owner::Team(team)) => {
                if team.contains_user(app, req_user)? {
                    Owner::Team(team)
                } else {
                    return Err(human(&format_args!(
                        "only members of {} can add it as \
                         an owner",
                        login
                    )));
                }
            }
            Err(err) => {
                if login.contains(':') {
                    Owner::Team(Team::create(app, conn, login, req_user)?)
                } else {
                    return Err(err);
                }
            }
        };

        let crate_owner = CrateOwner {
            crate_id: self.id,
            owner_id: owner.id(),
            created_by: req_user.id,
            owner_kind: owner.kind() as i32,
        };
        diesel::insert(&crate_owner.on_conflict(
            crate_owners::table.primary_key(),
            do_update().set(crate_owners::deleted.eq(false)),
        )).into(crate_owners::table)
            .execute(conn)?;

        Ok(())
    }

    pub fn owner_remove(
        &self,
        conn: &PgConnection,
        _req_user: &User,
        login: &str,
    ) -> CargoResult<()> {
        let owner = Owner::find_by_login(conn, login).map_err(|_| {
            human(&format_args!("could not find owner with login `{}`", login))
        })?;
        let target = crate_owners::table.find((self.id(), owner.id(), owner.kind() as i32));
        diesel::update(target)
            .set(crate_owners::deleted.eq(true))
            .execute(conn)?;
        Ok(())
    }

    pub fn add_version(
        &mut self,
        conn: &GenericConnection,
        ver: &semver::Version,
        features: &HashMap<String, Vec<String>>,
        authors: &[String],
    ) -> CargoResult<Version> {
        if Version::find_by_num(conn, self.id, ver)?.is_some() {
            return Err(human(
                &format_args!("crate version `{}` is already uploaded", ver),
            ));
        }
        Version::insert(conn, self.id, ver, features, authors)
    }

    pub fn keywords(&self, conn: &GenericConnection) -> CargoResult<Vec<Keyword>> {
        let stmt = conn.prepare(
            "SELECT keywords.* FROM keywords
                                      LEFT JOIN crates_keywords
                                      ON keywords.id = crates_keywords.keyword_id
                                      WHERE crates_keywords.crate_id = $1",
        )?;
        let rows = stmt.query(&[&self.id])?;
        Ok(rows.iter().map(|r| Model::from_row(&r)).collect())
    }

    pub fn categories(&self, conn: &GenericConnection) -> CargoResult<Vec<Category>> {
        let stmt = conn.prepare(
            "SELECT categories.* FROM categories \
             LEFT JOIN crates_categories \
             ON categories.id = \
             crates_categories.category_id \
             WHERE crates_categories.crate_id = $1",
        )?;
        let rows = stmt.query(&[&self.id])?;
        Ok(rows.iter().map(|r| Model::from_row(&r)).collect())
    }

    pub fn badges(&self, conn: &PgConnection) -> QueryResult<Vec<Badge>> {
        badges::table.filter(badges::crate_id.eq(self.id)).load(
            conn,
        )
    }

    /// Returns (dependency, dependent crate name, dependent crate downloads)
    pub fn reverse_dependencies(
        &self,
        conn: &PgConnection,
        offset: i64,
        limit: i64,
    ) -> CargoResult<(Vec<ReverseDependency>, i64)> {
        use diesel::expression::dsl::sql;
        use diesel::types::{Integer, Text, BigInt};

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

impl Model for Crate {
    fn from_row(row: &Row) -> Crate {
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
            license: row.get("license"),
            repository: row.get("repository"),
            max_upload_size: row.get("max_upload_size"),
        }
    }
    fn table_name(_: Option<Crate>) -> &'static str {
        "crates"
    }
}

/// Handles the `GET /crates` route.
#[allow(trivial_casts)]
pub fn index(req: &mut Request) -> CargoResult<Response> {
    use diesel::expression::dsl::sql;
    use diesel::types::{BigInt, Bool};

    let conn = req.db_conn()?;
    let (offset, limit) = req.pagination(10, 100)?;
    let params = req.query();
    let sort = params.get("sort").map(|s| &**s).unwrap_or("alpha");

    let mut query = crates::table
        .select((
            ALL_COLUMNS,
            sql::<BigInt>("COUNT(*) OVER ()"),
            sql::<Bool>("false"),
        ))
        .limit(limit)
        .offset(offset)
        .into_boxed();

    if sort == "downloads" {
        query = query.order(crates::downloads.desc())
    } else {
        query = query.order(crates::name.asc())
    }

    if let Some(q_string) = params.get("q") {
        let q = plainto_tsquery(q_string);
        query = query.filter(q.matches(crates::textsearchable_index_col).or(
            crates::name.eq(
                q_string,
            ),
        ));

        query = query.select((
            ALL_COLUMNS,
            sql::<BigInt>("COUNT(*) OVER()"),
            crates::name.eq(q_string),
        ));
        let perfect_match = crates::name.eq(q_string).desc();
        if sort == "downloads" {
            query = query.order((perfect_match, crates::downloads.desc()));
        } else {
            let rank = ts_rank_cd(crates::textsearchable_index_col, q);
            query = query.order((perfect_match, rank.desc()))
        }
    }

    if let Some(cat) = params.get("category") {
        query = query.filter(
            crates::id.eq_any(
                crates_categories::table
                    .select(crates_categories::crate_id)
                    .inner_join(categories::table)
                    .filter(categories::slug.eq(cat).or(categories::slug.like(format!(
                        "{}::%",
                        cat
                    )))),
            ),
        );
    }

    if let Some(kw) = params.get("keyword") {
        query = query.filter(
            crates::id.eq_any(
                crates_keywords::table
                    .select(crates_keywords::crate_id)
                    .inner_join(keywords::table)
                    .filter(::lower(keywords::keyword).eq(::lower(kw))),
            ),
        );
    } else if let Some(letter) = params.get("letter") {
        let pattern = format!(
            "{}%",
            letter
                .chars()
                .next()
                .unwrap()
                .to_lowercase()
                .collect::<String>()
        );
        query = query.filter(canon_crate_name(crates::name).like(pattern));
    } else if let Some(user_id) = params.get("user_id").and_then(|s| s.parse::<i32>().ok()) {
        query = query.filter(
            crates::id.eq_any(
                crate_owners::table
                    .select(crate_owners::crate_id)
                    .filter(crate_owners::owner_id.eq(user_id))
                    .filter(crate_owners::owner_kind.eq(OwnerKind::User as i32)),
            ),
        );
    } else if let Some(team_id) = params.get("team_id").and_then(|s| s.parse::<i32>().ok()) {
        query = query.filter(
            crates::id.eq_any(
                crate_owners::table
                    .select(crate_owners::crate_id)
                    .filter(crate_owners::owner_id.eq(team_id))
                    .filter(crate_owners::owner_kind.eq(OwnerKind::Team as i32)),
            ),
        );
    } else if params.get("following").is_some() {
        query = query.filter(crates::id.eq_any(
            follows::table.select(follows::crate_id).filter(
                follows::user_id.eq(req.user()?.id),
            ),
        ));
    }

    let data = query.load::<(Crate, i64, bool)>(&*conn)?;
    let total = data.get(0).map(|&(_, t, _)| t).unwrap_or(0);
    let crates = data.iter()
        .map(|&(ref c, _, _)| c.clone())
        .collect::<Vec<_>>();
    let perfect_matches = data.into_iter().map(|(_, _, b)| b).collect::<Vec<_>>();

    let versions = Version::belonging_to(&crates)
        .load::<Version>(&*conn)?
        .grouped_by(&crates)
        .into_iter()
        .map(|versions| Version::max(versions.into_iter().map(|v| v.num)));

    let crates = versions
        .zip(crates)
        .zip(perfect_matches)
        .map(|((max_version, krate), perfect_match)| {
            // FIXME: If we add crate_id to the Badge enum we can eliminate
            // this N+1
            let badges = badges::table
                .filter(badges::crate_id.eq(krate.id))
                .load::<Badge>(&*conn)?;
            Ok(krate.minimal_encodable(
                max_version,
                Some(badges),
                perfect_match,
            ))
        })
        .collect::<Result<_, ::diesel::result::Error>>()?;

    #[derive(RustcEncodable)]
    struct R {
        crates: Vec<EncodableCrate>,
        meta: Meta,
    }
    #[derive(RustcEncodable)]
    struct Meta {
        total: i64,
    }

    Ok(req.json(&R {
        crates: crates,
        meta: Meta { total: total },
    }))
}

/// Handles the `GET /summary` route.
pub fn summary(req: &mut Request) -> CargoResult<Response> {
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
                Ok(krate.minimal_encodable(max_version, None, false))
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

    #[derive(RustcEncodable)]
    struct R {
        num_downloads: i64,
        num_crates: i64,
        new_crates: Vec<EncodableCrate>,
        most_downloaded: Vec<EncodableCrate>,
        just_updated: Vec<EncodableCrate>,
        popular_keywords: Vec<EncodableKeyword>,
        popular_categories: Vec<EncodableCategory>,
    }
    Ok(req.json(&R {
        num_downloads: num_downloads,
        num_crates: num_crates,
        new_crates: encode_crates(new_crates)?,
        most_downloaded: encode_crates(most_downloaded)?,
        just_updated: encode_crates(just_updated)?,
        popular_keywords: popular_keywords,
        popular_categories: popular_categories,
    }))
}

/// Handles the `GET /crates/:crate_id` route.
pub fn show(req: &mut Request) -> CargoResult<Response> {
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

    let badges = badges::table.filter(badges::crate_id.eq(krate.id)).load(
        &*conn,
    )?;
    let max_version = krate.max_version(&conn)?;

    #[derive(RustcEncodable)]
    struct R {
        krate: EncodableCrate,
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

/// Handles the `PUT /crates/new` route.
pub fn new(req: &mut Request) -> CargoResult<Response> {
    let app = req.app().clone();
    let (new_crate, user) = parse_new_headers(req)?;

    let name = &*new_crate.name;
    let vers = &*new_crate.vers;
    let features = new_crate
        .features
        .iter()
        .map(|(k, v)| {
            (
                k[..].to_string(),
                v.iter().map(|v| v[..].to_string()).collect(),
            )
        })
        .collect::<HashMap<String, Vec<String>>>();
    let keywords = new_crate
        .keywords
        .as_ref()
        .map(|kws| kws.iter().map(|kw| &**kw).collect())
        .unwrap_or_else(Vec::new);

    let categories = new_crate.categories.as_ref().map(|s| &s[..]).unwrap_or(&[]);
    let categories: Vec<_> = categories.iter().map(|k| &**k).collect();

    let conn = req.db_conn()?;
    conn.transaction(|| {
        // Persist the new crate, if it doesn't already exist
        let persist = NewCrate {
            name: name,
            description: new_crate.description.as_ref().map(|s| &**s),
            homepage: new_crate.homepage.as_ref().map(|s| &**s),
            documentation: new_crate.documentation.as_ref().map(|s| &**s),
            readme: new_crate.readme.as_ref().map(|s| &**s),
            repository: new_crate.repository.as_ref().map(|s| &**s),
            license: new_crate.license.as_ref().map(|s| &**s),
            max_upload_size: None,
        };

        let license_file = new_crate.license_file.as_ref().map(|s| &**s);
        let krate = persist.create_or_update(&conn, license_file, user.id)?;

        let owners = krate.owners(&conn)?;
        if rights(req.app(), &owners, &user)? < Rights::Publish {
            return Err(human(
                "crate name has already been claimed by \
                 another user",
            ));
        }

        if krate.name != name {
            return Err(human(
                &format_args!("crate was previously named `{}`", krate.name),
            ));
        }

        let length = req.content_length().chain_error(|| {
            human("missing header: Content-Length")
        })?;
        let max = krate.max_upload_size.map(|m| m as u64).unwrap_or(
            app.config
                .max_upload_size,
        );
        if length > max {
            return Err(human(&format_args!("max upload size is: {}", max)));
        }

        // This is only redundant for now. Eventually the duplication will be removed.
        let license = new_crate.license.clone();

        // Persist the new version of this crate
        let version = NewVersion::new(krate.id, vers, &features, license, license_file)?
            .save(&conn, &new_crate.authors)?;

        // Link this new version to all dependencies
        let git_deps = dependency::add_dependencies(&conn, &new_crate.deps, version.id)?;

        // Update all keywords for this crate
        Keyword::update_crate(&conn, &krate, &keywords)?;

        // Update all categories for this crate, collecting any invalid categories
        // in order to be able to warn about them
        let ignored_invalid_categories = Category::update_crate(&conn, &krate, &categories)?;

        // Update all badges for this crate, collecting any invalid badges in
        // order to be able to warn about them
        let ignored_invalid_badges = Badge::update_crate(&conn, &krate, new_crate.badges.as_ref())?;
        let max_version = krate.max_version(&conn)?;

        // Upload the crate, return way to delete the crate from the server
        // If the git commands fail below, we shouldn't keep the crate on the
        // server.
        let (cksum, mut bomb) = app.config.uploader.upload(req, &krate, max, vers)?;

        // Register this crate in our local git repo.
        let git_crate = git::Crate {
            name: name.to_string(),
            vers: vers.to_string(),
            cksum: cksum.to_hex(),
            features: features,
            deps: git_deps,
            yanked: Some(false),
        };
        git::add_crate(&**req.app(), &git_crate).chain_error(|| {
            internal(&format_args!(
                "could not add crate `{}` to the git repo",
                name
            ))
        })?;

        // Now that we've come this far, we're committed!
        bomb.path = None;

        #[derive(RustcEncodable)]
        struct Warnings<'a> {
            invalid_categories: Vec<&'a str>,
            invalid_badges: Vec<&'a str>,
        }
        let warnings = Warnings {
            invalid_categories: ignored_invalid_categories,
            invalid_badges: ignored_invalid_badges,
        };

        #[derive(RustcEncodable)]
        struct R<'a> {
            krate: EncodableCrate,
            warnings: Warnings<'a>,
        }
        Ok(req.json(&R {
            krate: krate.minimal_encodable(max_version, None, false),
            warnings: warnings,
        }))
    })
}

fn parse_new_headers(req: &mut Request) -> CargoResult<(upload::NewCrate, User)> {
    // Read the json upload request
    let amt = read_le_u32(req.body())? as u64;
    let max = req.app().config.max_upload_size;
    if amt > max {
        return Err(human(&format_args!("max upload size is: {}", max)));
    }
    let mut json = vec![0; amt as usize];
    read_fill(req.body(), &mut json)?;
    let json = String::from_utf8(json).map_err(|_| {
        human("json body was not valid utf-8")
    })?;
    let new: upload::NewCrate = json::decode(&json).map_err(|e| {
        human(&format_args!("invalid upload request: {:?}", e))
    })?;

    // Make sure required fields are provided
    fn empty(s: Option<&String>) -> bool {
        s.map_or(true, |s| s.is_empty())
    }
    let mut missing = Vec::new();

    if empty(new.description.as_ref()) {
        missing.push("description");
    }
    if empty(new.license.as_ref()) && empty(new.license_file.as_ref()) {
        missing.push("license");
    }
    if new.authors.iter().all(|s| s.is_empty()) {
        missing.push("authors");
    }
    if !missing.is_empty() {
        return Err(human(&format_args!(
            "missing or empty metadata fields: {}. Please \
             see http://doc.crates.io/manifest.html#package-metadata for \
             how to upload metadata",
            missing.join(", ")
        )));
    }

    let user = req.user()?;
    Ok((new, user.clone()))
}

/// Handles the `GET /crates/:crate_id/:version/download` route.
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
        #[derive(RustcEncodable)]
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
        .filter(crate_id.eq_any(
            Crate::by_name(crate_name).select(crates::id),
        ))
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

    #[derive(RustcEncodable, Queryable)]
    struct ExtraDownload {
        date: String,
        downloads: i64,
    }
    #[derive(RustcEncodable)]
    struct R {
        version_downloads: Vec<EncodableVersionDownload>,
        meta: Meta,
    }
    #[derive(RustcEncodable)]
    struct Meta {
        extra_downloads: Vec<ExtraDownload>,
    }
    let meta = Meta { extra_downloads: extra };
    Ok(req.json(&R {
        version_downloads: downloads,
        meta: meta,
    }))
}

#[derive(Insertable, Queryable, Identifiable, Associations)]
#[belongs_to(User)]
#[primary_key(user_id, crate_id)]
#[table_name = "follows"]
pub struct Follow {
    user_id: i32,
    crate_id: i32,
}

fn follow_target(req: &mut Request) -> CargoResult<Follow> {
    let user = req.user()?;
    let conn = req.db_conn()?;
    let crate_name = &req.params()["crate_id"];
    let crate_id = Crate::by_name(crate_name).select(crates::id).first(&*conn)?;
    Ok(Follow {
        user_id: user.id,
        crate_id: crate_id,
    })
}

/// Handles the `PUT /crates/:crate_id/follow` route.
pub fn follow(req: &mut Request) -> CargoResult<Response> {
    let follow = follow_target(req)?;
    let conn = req.db_conn()?;
    diesel::insert(&follow.on_conflict_do_nothing())
        .into(follows::table)
        .execute(&*conn)?;
    #[derive(RustcEncodable)]
    struct R {
        ok: bool,
    }
    Ok(req.json(&R { ok: true }))
}

/// Handles the `DELETE /crates/:crate_id/follow` route.
pub fn unfollow(req: &mut Request) -> CargoResult<Response> {
    let follow = follow_target(req)?;
    let conn = req.db_conn()?;
    diesel::delete(&follow).execute(&*conn)?;
    #[derive(RustcEncodable)]
    struct R {
        ok: bool,
    }
    Ok(req.json(&R { ok: true }))
}

/// Handles the `GET /crates/:crate_id/following` route.
pub fn following(req: &mut Request) -> CargoResult<Response> {
    use diesel::expression::dsl::exists;

    let follow = follow_target(req)?;
    let conn = req.db_conn()?;
    let following = diesel::select(exists(follows::table.find(follow.id())))
        .get_result(&*conn)?;
    #[derive(RustcEncodable)]
    struct R {
        following: bool,
    }
    Ok(req.json(&R { following: following }))
}

/// Handles the `GET /crates/:crate_id/versions` route.
// FIXME: Not sure why this is necessary since /crates/:crate_id returns
// this information already, but ember is definitely requesting it
pub fn versions(req: &mut Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let tx = req.tx()?;
    let krate = Crate::find_by_name(tx, crate_name)?;
    let versions = krate.versions(tx)?;
    let versions = versions
        .into_iter()
        .map(|v| v.encodable(crate_name))
        .collect();

    #[derive(RustcEncodable)]
    struct R {
        versions: Vec<EncodableVersion>,
    }
    Ok(req.json(&R { versions: versions }))
}

/// Handles the `GET /crates/:crate_id/owners` route.
pub fn owners(req: &mut Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(crate_name).first::<Crate>(&*conn)?;
    let owners = krate
        .owners(&conn)?
        .into_iter()
        .map(Owner::encodable)
        .collect();

    #[derive(RustcEncodable)]
    struct R {
        users: Vec<EncodableOwner>,
    }
    Ok(req.json(&R { users: owners }))
}

/// Handles the `GET /crates/:crate_id/owner_team` route.
pub fn owner_team(req: &mut Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(crate_name).first::<Crate>(&*conn)?;
    let owners = Team::owning(&krate, &conn)?
        .into_iter()
        .map(Owner::encodable)
        .collect();

    #[derive(RustcEncodable)]
    struct R {
        teams: Vec<EncodableOwner>,
    }
    Ok(req.json(&R { teams: owners }))
}

/// Handles the `GET /crates/:crate_id/owner_user` route.
pub fn owner_user(req: &mut Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(crate_name).first::<Crate>(&*conn)?;
    let owners = User::owning(&krate, &conn)?
        .into_iter()
        .map(Owner::encodable)
        .collect();

    #[derive(RustcEncodable)]
    struct R {
        users: Vec<EncodableOwner>,
    }
    Ok(req.json(&R { users: owners }))
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
    req.body().read_to_string(&mut body)?;
    let user = req.user()?;
    let conn = req.db_conn()?;
    let krate = Crate::by_name(&req.params()["crate_id"]).first::<Crate>(
        &*conn,
    )?;
    let owners = krate.owners(&conn)?;

    match rights(req.app(), &owners, user)? {
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

    let request: Request = json::decode(&body).map_err(
        |_| human("invalid json request"),
    )?;

    let logins = request.owners.or(request.users).ok_or_else(|| {
        human("invalid json request")
    })?;

    for login in &logins {
        if add {
            if owners.iter().any(|owner| owner.login() == *login) {
                return Err(human(&format_args!("`{}` is already an owner", login)));
            }
            krate.owner_add(req.app(), &conn, user, login)?;
        } else {
            // Removing the team that gives you rights is prevented because
            // team members only have Rights::Publish
            if *login == user.gh_login {
                return Err(human("cannot remove yourself as an owner"));
            }
            krate.owner_remove(&conn, user, login)?;
        }
    }

    #[derive(RustcEncodable)]
    struct R {
        ok: bool,
    }
    Ok(req.json(&R { ok: true }))
}

/// Handles the `GET /crates/:crate_id/reverse_dependencies` route.
pub fn reverse_dependencies(req: &mut Request) -> CargoResult<Response> {
    let name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(name).first::<Crate>(&*conn)?;
    let (offset, limit) = req.pagination(10, 100)?;
    let (rev_deps, total) = krate.reverse_dependencies(&*conn, offset, limit)?;
    let rev_deps = rev_deps
        .into_iter()
        .map(ReverseDependency::encodable)
        .collect();

    #[derive(RustcEncodable)]
    struct R {
        dependencies: Vec<EncodableDependency>,
        meta: Meta,
    }
    #[derive(RustcEncodable)]
    struct Meta {
        total: i64,
    }
    Ok(req.json(&R {
        dependencies: rev_deps,
        meta: Meta { total: total },
    }))
}

use diesel::types::{Text, Date};
sql_function!(canon_crate_name, canon_crate_name_t, (x: Text) -> Text);
sql_function!(to_char, to_char_t, (a: Date, b: Text) -> Text);
