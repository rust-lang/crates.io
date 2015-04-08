use std::ascii::AsciiExt;
use std::collections::HashMap;
use time::Timespec;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use pg;
use pg::types::Slice;

use {Model, Crate};
use db::{Connection, RequestTransaction};
use util::{RequestUtils, CargoResult, ChainError, internal};
use util::errors::NotFound;

#[derive(Clone)]
pub struct Keyword {
    pub id: i32,
    pub keyword: String,
    pub created_at: Timespec,
    pub crates_cnt: i32,
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct EncodableKeyword {
    pub id: String,
    pub keyword: String,
    pub created_at: String,
    pub crates_cnt: i32,
}

impl Keyword {
    pub fn find(conn: &Connection, id: i32) -> CargoResult<Keyword> {
        Model::find(conn, id)
    }

    pub fn find_by_keyword(conn: &Connection, name: &str)
                           -> CargoResult<Option<Keyword>> {
        let stmt = try!(conn.prepare("SELECT * FROM keywords \
                                      WHERE keyword = $1"));
        let rows = try!(stmt.query(&[&name]));
        Ok(rows.iter().next().map(|r| Model::from_row(&r)))
    }

    pub fn find_or_insert(conn: &Connection, name: &str)
                          -> CargoResult<Keyword> {
        // TODO: racy (the select then insert is not atomic)
        let stmt = try!(conn.prepare("SELECT * FROM keywords
                                      WHERE keyword = $1"));
        for row in try!(stmt.query(&[&name])) {
            return Ok(Model::from_row(&row))
        }

        let stmt = try!(conn.prepare("INSERT INTO keywords \
                                      (keyword, created_at, crates_cnt)
                                      VALUES ($1, $2, 0) \
                                      RETURNING *"));
        let now = ::now();
        let rows = try!(stmt.query(&[&name, &now]));
        Ok(Model::from_row(&try!(rows.iter().next().chain_error(|| {
            internal("no version returned")
        }))))
    }

    pub fn valid_name(name: &str) -> bool {
        if name.len() == 0 { return false }
        name.chars().next().unwrap().is_alphanumeric() &&
            name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') &&
            name.chars().all(|c| c.is_ascii())
    }

    pub fn encodable(self) -> EncodableKeyword {
        let Keyword { id: _, crates_cnt, keyword, created_at } = self;
        EncodableKeyword {
            id: keyword.clone(),
            created_at: ::encode_time(created_at),
            crates_cnt: crates_cnt,
            keyword: keyword,
        }
    }

    pub fn update_crate(conn: &Connection, krate: &Crate,
                        keywords: &[String]) -> CargoResult<()> {
        let old_kws = try!(krate.keywords(conn));
        let old_kws = old_kws.iter().map(|kw| {
            (&kw.keyword[..], kw)
        }).collect::<HashMap<_, _>>();
        let new_kws = try!(keywords.iter().map(|k| {
            let kw = try!(Keyword::find_or_insert(conn, &k));
            Ok((&k[..], kw))
        }).collect::<CargoResult<HashMap<_, _>>>());

        let to_rm = old_kws.iter().filter(|&(kw, _)| {
            !new_kws.contains_key(kw)
        }).map(|(_, v)| v.id).collect::<Vec<_>>();
        let to_add = new_kws.iter().filter(|&(kw, _)| {
            !old_kws.contains_key(kw)
        }).map(|(_, v)| v.id).collect::<Vec<_>>();

        if to_rm.len() > 0 {
            try!(conn.execute("UPDATE keywords
                                  SET crates_cnt = crates_cnt - 1
                                WHERE id = ANY($1)",
                              &[&Slice(&to_rm)]));
            try!(conn.execute("DELETE FROM crates_keywords
                                WHERE keyword_id = ANY($1)
                                  AND crate_id = $2",
                              &[&Slice(&to_rm), &krate.id]));
        }

        if to_add.len() > 0 {
            try!(conn.execute("UPDATE keywords
                                  SET crates_cnt = crates_cnt + 1
                                WHERE id = ANY($1)",
                              &[&Slice(&to_add)]));
            let insert = to_add.iter().map(|id| {
                let crate_id: i32 = krate.id;
                let id: i32 = *id;
                format!("({}, {})", crate_id,  id)
            }).collect::<Vec<_>>().connect(", ");
            try!(conn.execute(&format!("INSERT INTO crates_keywords
                                        (crate_id, keyword_id) VALUES {}",
                                       insert),
                              &[]));
        }

        Ok(())
    }
}

impl Model for Keyword {
    fn from_row(row: &pg::Row) -> Keyword {
        Keyword {
            id: row.get("id"),
            created_at: row.get("created_at"),
            crates_cnt: row.get("crates_cnt"),
            keyword: row.get("keyword"),
        }
    }
    fn table_name(_: Option<Keyword>) -> &'static str { "keywords" }
}

pub fn index(req: &mut Request) -> CargoResult<Response> {
    let conn = try!(req.tx());
    let (offset, limit) = try!(req.pagination(10, 100));
    let query = req.query();
    let sort = query.get("sort").map(|s| &s[..]).unwrap_or("alpha");
    let sort_sql = match sort {
        "crates" => "ORDER BY crates_cnt DESC",
        _ => "ORDER BY keyword ASC",
    };

    // Collect all the keywords
    let stmt = try!(conn.prepare(&format!("SELECT * FROM keywords {}
                                           LIMIT $1 OFFSET $2",
                                          sort_sql)));
    let mut keywords = Vec::new();
    for row in try!(stmt.query(&[&limit, &offset])) {
        let keyword: Keyword = Model::from_row(&row);
        keywords.push(keyword.encodable());
    }

    // Query for the total count of keywords
    let stmt = try!(conn.prepare("SELECT COUNT(*) FROM keywords"));
    let row = try!(stmt.query(&[])).into_iter().next().unwrap();
    let total = row.get(0);

    #[derive(RustcEncodable)]
    struct R { keywords: Vec<EncodableKeyword>, meta: Meta }
    #[derive(RustcEncodable)]
    struct Meta { total: i64 }

    Ok(req.json(&R {
        keywords: keywords,
        meta: Meta { total: total },
    }))
}

pub fn show(req: &mut Request) -> CargoResult<Response> {
    let name = &req.params()["keyword_id"];
    let conn = try!(req.tx());
    let kw = try!(Keyword::find_by_keyword(&*conn, &name));
    let kw = try!(kw.chain_error(|| NotFound));

    #[derive(RustcEncodable)]
    struct R { keyword: EncodableKeyword }
    Ok(req.json(&R { keyword: kw.encodable() }))
}

