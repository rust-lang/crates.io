use std::ascii::AsciiExt;
use time::Timespec;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel;
use pg::rows::Row;

use {Model, Crate};
use db::RequestTransaction;
use schema::*;
use util::{RequestUtils, CargoResult};

#[derive(Clone, Identifiable, Queryable, Debug)]
pub struct Keyword {
    pub id: i32,
    pub keyword: String,
    pub crates_cnt: i32,
    pub created_at: Timespec,
}

#[derive(Associations, Insertable, Identifiable, Debug)]
#[belongs_to(Keyword)]
#[belongs_to(Crate)]
#[table_name = "crates_keywords"]
#[primary_key(crate_id, keyword_id)]
pub struct CrateKeyword {
    crate_id: i32,
    keyword_id: i32,
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct EncodableKeyword {
    pub id: String,
    pub keyword: String,
    pub created_at: String,
    pub crates_cnt: i32,
}

impl Keyword {
    pub fn find_by_keyword(conn: &PgConnection, name: &str) -> QueryResult<Keyword> {
        keywords::table
            .filter(keywords::keyword.eq(::lower(name)))
            .first(&*conn)
    }

    pub fn find_or_create_all(conn: &PgConnection, names: &[&str]) -> QueryResult<Vec<Keyword>> {
        use diesel::pg::upsert::*;
        use diesel::expression::dsl::any;

        #[derive(Insertable)]
        #[table_name = "keywords"]
        struct NewKeyword<'a> {
            keyword: &'a str,
        }

        let lowercase_names: Vec<_> = names.iter().map(|s| s.to_lowercase()).collect();

        let new_keywords: Vec<_> = lowercase_names
            .iter()
            .map(|s| NewKeyword { keyword: s })
            .collect();

        // https://github.com/diesel-rs/diesel/issues/797
        if !new_keywords.is_empty() {
            diesel::insert(&new_keywords.on_conflict_do_nothing())
                .into(keywords::table)
                .execute(conn)?;
        }
        keywords::table
            .filter(::lower(keywords::keyword).eq(any(&lowercase_names)))
            .load(conn)
    }

    pub fn valid_name(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        name.chars().next().unwrap().is_alphanumeric() &&
            name.chars().all(
                |c| c.is_alphanumeric() || c == '_' || c == '-',
            ) && name.chars().all(|c| c.is_ascii())
    }

    pub fn encodable(self) -> EncodableKeyword {
        let Keyword {
            crates_cnt,
            keyword,
            created_at,
            ..
        } = self;
        EncodableKeyword {
            id: keyword.clone(),
            created_at: ::encode_time(created_at),
            crates_cnt: crates_cnt,
            keyword: keyword,
        }
    }

    pub fn update_crate(conn: &PgConnection, krate: &Crate, keywords: &[&str]) -> QueryResult<()> {
        conn.transaction(|| {
            let keywords = Keyword::find_or_create_all(conn, keywords)?;
            diesel::delete(CrateKeyword::belonging_to(krate)).execute(
                conn,
            )?;
            let crate_keywords = keywords
                .into_iter()
                .map(|kw| {
                    CrateKeyword {
                        crate_id: krate.id,
                        keyword_id: kw.id,
                    }
                })
                .collect::<Vec<_>>();
            diesel::insert(&crate_keywords)
                .into(crates_keywords::table)
                .execute(conn)?;
            Ok(())
        })
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
    fn table_name(_: Option<Keyword>) -> &'static str {
        "keywords"
    }
}

/// Handles the `GET /keywords` route.
pub fn index(req: &mut Request) -> CargoResult<Response> {
    use diesel::expression::dsl::sql;
    use diesel::types::BigInt;
    use schema::keywords;

    let conn = req.db_conn()?;
    let (offset, limit) = req.pagination(10, 100)?;
    let query = req.query();
    let sort = query.get("sort").map(|s| &s[..]).unwrap_or("alpha");

    let mut query = keywords::table
        .select((keywords::all_columns, sql::<BigInt>("COUNT(*) OVER ()")))
        .limit(limit)
        .offset(offset)
        .into_boxed();

    if sort == "crates" {
        query = query.order(keywords::crates_cnt.desc());
    } else {
        query = query.order(keywords::keyword.asc());
    }

    let data = query.load::<(Keyword, i64)>(&*conn)?;
    let total = data.get(0).map(|&(_, t)| t).unwrap_or(0);
    let kws = data.into_iter()
        .map(|(k, _)| k.encodable())
        .collect::<Vec<_>>();

    #[derive(RustcEncodable)]
    struct R {
        keywords: Vec<EncodableKeyword>,
        meta: Meta,
    }
    #[derive(RustcEncodable)]
    struct Meta {
        total: i64,
    }

    Ok(req.json(&R {
        keywords: kws,
        meta: Meta { total: total },
    }))
}

/// Handles the `GET /keywords/:keyword_id` route.
pub fn show(req: &mut Request) -> CargoResult<Response> {
    let name = &req.params()["keyword_id"];
    let conn = req.db_conn()?;

    let kw = Keyword::find_by_keyword(&conn, name)?;

    #[derive(RustcEncodable)]
    struct R {
        keyword: EncodableKeyword,
    }
    Ok(req.json(&R { keyword: kw.encodable() }))
}
