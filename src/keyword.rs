use std::ascii::AsciiExt;
use std::collections::HashMap;
use time::Timespec;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel;
use pg::GenericConnection;
use pg::rows::Row;

use {Model, Crate};
use db::RequestTransaction;
use schema::*;
use util::{RequestUtils, CargoResult, ChainError, internal};
use util::errors::NotFound;

#[derive(Clone, Identifiable, Queryable)]
pub struct Keyword {
    pub id: i32,
    pub keyword: String,
    pub crates_cnt: i32,
    pub created_at: Timespec,
}

#[derive(Associations, Insertable, Identifiable)]
#[belongs_to(Keyword)]
#[belongs_to(Crate)]
#[table_name="crates_keywords"]
#[primary_key(crate_id, keyword_id)]
pub struct CrateKeyword {
    crate_id: i32,
    keyword_id: i32,
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct EncodableKeyword {
    pub id: String,
    pub keyword: String,
    pub created_at: String,
    pub crates_cnt: i32,
}

impl Keyword {
    pub fn find_by_keyword(conn: &GenericConnection, name: &str)
                           -> CargoResult<Option<Keyword>> {
        let stmt = conn.prepare("SELECT * FROM keywords \
                                      WHERE keyword = LOWER($1)")?;
        let rows = stmt.query(&[&name])?;
        Ok(rows.iter().next().map(|r| Model::from_row(&r)))
    }

    pub fn find_or_create_all(conn: &PgConnection, names: &[&str]) -> QueryResult<Vec<Keyword>> {
        use diesel::pg::upsert::*;
        use diesel::expression::dsl::any;

        #[derive(Insertable)]
        #[table_name="keywords"]
        struct NewKeyword<'a> {
            keyword: &'a str,
        }

        let (lowercase_names, new_keywords): (Vec<_>, Vec<_>) = names.iter()
            .map(|s| (s.to_lowercase(), NewKeyword { keyword: *s }))
            .unzip();

        // https://github.com/diesel-rs/diesel/issues/797
        if !new_keywords.is_empty() {
            diesel::insert(&new_keywords.on_conflict_do_nothing()).into(keywords::table)
                .execute(conn)?;
        }
        keywords::table.filter(::lower(keywords::keyword).eq(any(lowercase_names)))
            .load(conn)
    }

    pub fn find_or_insert(conn: &GenericConnection, name: &str)
                          -> CargoResult<Keyword> {
        // TODO: racy (the select then insert is not atomic)
        let stmt = conn.prepare("SELECT * FROM keywords
                                      WHERE keyword = LOWER($1)")?;
        for row in stmt.query(&[&name])?.iter() {
            return Ok(Model::from_row(&row))
        }

        let stmt = conn.prepare("INSERT INTO keywords (keyword) VALUES (LOWER($1))
                                      RETURNING *")?;
        let rows = stmt.query(&[&name])?;
        Ok(Model::from_row(&rows.iter().next().chain_error(|| {
            internal("no version returned")
        })?))
    }

    pub fn all(conn: &GenericConnection, sort: &str, limit: i64, offset: i64)
               -> CargoResult<Vec<Keyword>> {

        let sort_sql = match sort {
           "crates" => "ORDER BY crates_cnt DESC",
           _ => "ORDER BY keyword ASC",
        };

        let stmt = conn.prepare(&format!("SELECT * FROM keywords {}
                                               LIMIT $1 OFFSET $2",
                                         sort_sql))?;

        let keywords: Vec<_> = stmt.query(&[&limit, &offset])?
            .iter()
            .map(|row| Model::from_row(&row))
            .collect();

        Ok(keywords)
    }

    pub fn valid_name(name: &str) -> bool {
        if name.is_empty() { return false }
        name.chars().next().unwrap().is_alphanumeric() &&
            name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') &&
            name.chars().all(|c| c.is_ascii())
    }

    pub fn encodable(self) -> EncodableKeyword {
        let Keyword { crates_cnt, keyword, created_at, .. } = self;
        EncodableKeyword {
            id: keyword.clone(),
            created_at: ::encode_time(created_at),
            crates_cnt: crates_cnt,
            keyword: keyword,
        }
    }

    pub fn update_crate(conn: &PgConnection,
                        krate: &Crate,
                        keywords: &[&str]) -> QueryResult<()> {
        conn.transaction(|| {
            let keywords = Keyword::find_or_create_all(conn, keywords)?;
            diesel::delete(CrateKeyword::belonging_to(krate))
                .execute(conn)?;
            let crate_keywords = keywords.into_iter().map(|kw| {
                CrateKeyword { crate_id: krate.id, keyword_id: kw.id }
            }).collect::<Vec<_>>();
            diesel::insert(&crate_keywords).into(crates_keywords::table)
                .execute(conn)?;
            Ok(())
        })
    }

    pub fn update_crate_old(conn: &GenericConnection,
                        krate: &Crate,
                        keywords: &[String]) -> CargoResult<()> {
        let old_kws = krate.keywords(conn)?;
        let old_kws = old_kws.iter().map(|kw| {
            (&kw.keyword[..], kw)
        }).collect::<HashMap<_, _>>();
        let new_kws = keywords.iter().map(|k| {
            let kw = Keyword::find_or_insert(conn, k)?;
            Ok((k.as_str(), kw))
        }).collect::<CargoResult<HashMap<_, _>>>()?;

        let to_rm = old_kws.iter().filter(|&(kw, _)| {
            !new_kws.contains_key(kw)
        }).map(|(_, v)| v.id).collect::<Vec<_>>();
        let to_add = new_kws.iter().filter(|&(kw, _)| {
            !old_kws.contains_key(kw)
        }).map(|(_, v)| v.id).collect::<Vec<_>>();

        if !to_rm.is_empty() {
            conn.execute("DELETE FROM crates_keywords
                                WHERE keyword_id = ANY($1)
                                  AND crate_id = $2",
                         &[&to_rm, &krate.id])?;
        }

        if !to_add.is_empty() {
            let insert = to_add.iter().map(|id| {
                let crate_id: i32 = krate.id;
                let id: i32 = *id;
                format!("({}, {})", crate_id,  id)
            }).collect::<Vec<_>>().join(", ");
            conn.execute(&format!("INSERT INTO crates_keywords
                                        (crate_id, keyword_id) VALUES {}",
                                  insert),
                         &[])?;
        }

        Ok(())
    }
}

impl Model for Keyword {
    fn from_row(row: &Row) -> Keyword {
        Keyword {
            id: row.get("id"),
            created_at: row.get("created_at"),
            crates_cnt: row.get("crates_cnt"),
            keyword: row.get("keyword"),
        }
    }
    fn table_name(_: Option<Keyword>) -> &'static str { "keywords" }
}

/// Handles the `GET /keywords` route.
pub fn index(req: &mut Request) -> CargoResult<Response> {
    let conn = req.tx()?;
    let (offset, limit) = req.pagination(10, 100)?;
    let query = req.query();
    let sort = query.get("sort").map(|s| &s[..]).unwrap_or("alpha");

    let keywords = Keyword::all(conn, sort, limit, offset)?;
    let keywords = keywords.into_iter().map(Keyword::encodable).collect();

    // Query for the total count of keywords
    let total = Keyword::count(conn)?;

    #[derive(RustcEncodable)]
    struct R { keywords: Vec<EncodableKeyword>, meta: Meta }
    #[derive(RustcEncodable)]
    struct Meta { total: i64 }

    Ok(req.json(&R {
        keywords: keywords,
        meta: Meta { total: total },
    }))
}

/// Handles the `GET /keywords/:keyword_id` route.
pub fn show(req: &mut Request) -> CargoResult<Response> {
    let name = &req.params()["keyword_id"];
    let conn = req.tx()?;
    let kw = Keyword::find_by_keyword(conn, name)?;
    let kw = kw.chain_error(|| NotFound)?;

    #[derive(RustcEncodable)]
    struct R { keyword: EncodableKeyword }
    Ok(req.json(&R { keyword: kw.encodable() }))
}
