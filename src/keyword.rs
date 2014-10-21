use std::collections::HashMap;
use time::Timespec;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use pg::PostgresRow;
use pg::types::ToSql;

use {Model, Crate};
use db::{Connection, RequestTransaction};
use util::{RequestUtils, CargoResult, Require, internal};
use util::errors::{NotFound, CargoError};

#[deriving(Clone)]
pub struct Keyword {
    pub id: i32,
    pub keyword: String,
    pub created_at: Timespec,
    pub crates_cnt: i32,
}

#[deriving(Encodable, Decodable)]
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
        let mut rows = try!(stmt.query(&[&name as &ToSql]));
        Ok(rows.next().map(|r| Model::from_row(&r)))
    }

    pub fn find_or_insert(conn: &Connection, name: &str)
                          -> CargoResult<Keyword> {
        // TODO: racy (the select then insert is not atomic)
        let stmt = try!(conn.prepare("SELECT * FROM keywords
                                      WHERE keyword = $1"));
        for row in try!(stmt.query(&[&name as &ToSql])) {
            return Ok(Model::from_row(&row))
        }

        let stmt = try!(conn.prepare("INSERT INTO keywords \
                                      (keyword, created_at, crates_cnt)
                                      VALUES ($1, $2, 0) \
                                      RETURNING *"));
        let now = ::now();
        let mut rows = try!(stmt.query(&[&name as &ToSql, &now]));
        Ok(Model::from_row(&try!(rows.next().require(|| {
            internal("no version returned")
        }))))
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
        let new_kws = try!(keywords.iter().map(|k| {
            let kw = try!(Keyword::find_or_insert(conn, k.as_slice()));
            Ok((k.as_slice(), kw))
        }).collect::<CargoResult<HashMap<_, _>>>());
        let old_kws = try!(krate.keywords(conn));
        let old_kws = old_kws.iter().map(|kw| {
            (kw.keyword.as_slice(), kw)
        }).collect::<HashMap<_, _>>();

        let to_rm = old_kws.iter().filter(|&(kw, _)| {
            !new_kws.contains_key(kw)
        }).map(|(_, v)| v.id).collect::<Vec<_>>();
        let to_add = new_kws.iter().filter(|&(kw, _)| {
            !old_kws.contains_key(kw)
        }).map(|(_, v)| v.id).collect::<Vec<_>>();

        if to_rm.len() > 0 {
            try!(conn.execute(format!("UPDATE keywords
                                          SET crates_cnt = crates_cnt - 1
                                        WHERE id IN ({:#})", to_rm).as_slice(),
                              &[]));
            try!(conn.execute(format!("DELETE FROM crates_keywords
                                        WHERE keyword_id IN ({:#})
                                          AND crate_id = $1", to_rm).as_slice(),
                              &[&krate.id]));
        }

        if to_add.len() > 0 {
            try!(conn.execute(format!("UPDATE keywords
                                          SET crates_cnt = crates_cnt + 1
                                        WHERE id IN ({:#})", to_add).as_slice(),
                              &[]));
            let insert = to_add.iter().map(|id| {
                let crate_id: i32 = krate.id;
                let id: i32 = *id;
                format!("({}, {})", crate_id,  id)
            }).collect::<Vec<_>>().connect(", ");
            try!(conn.execute(format!("INSERT INTO crates_keywords
                                       (crate_id, keyword_id) VALUES {}",
                                      insert).as_slice(),
                              &[]));
        }

        Ok(())
    }
}

impl Model for Keyword {
    fn from_row(row: &PostgresRow) -> Keyword {
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
    let sort = query.find_equiv(&"sort").map(|s| s.as_slice()).unwrap_or("alpha");
    let sort_sql = match sort {
        "crates" => "ORDER BY crates_cnt DESC",
        _ => "ORDER BY keyword ASC",
    };

    // Collect all the keywords
    let stmt = try!(conn.prepare(format!("SELECT * FROM keywords {}
                                          LIMIT $1 OFFSET $2",
                                         sort_sql).as_slice()));
    let mut keywords = Vec::new();
    for row in try!(stmt.query(&[&limit, &offset])) {
        let keyword: Keyword = Model::from_row(&row);
        keywords.push(keyword.encodable());
    }

    // Query for the total count of keywords
    let stmt = try!(conn.prepare("SELECT COUNT(*) FROM keywords"));
    let row = try!(stmt.query(&[])).next().unwrap();
    let total = row.get(0u);

    #[deriving(Encodable)]
    struct R { keywords: Vec<EncodableKeyword>, meta: Meta }
    #[deriving(Encodable)]
    struct Meta { total: i64 }

    Ok(req.json(&R {
        keywords: keywords,
        meta: Meta { total: total },
    }))
}

pub fn show(req: &mut Request) -> CargoResult<Response> {
    let name = &req.params()["keyword_id"];
    let conn = try!(req.tx());
    let kw = match try!(Keyword::find_by_keyword(&*conn, name.as_slice())) {
        Some(kw) => kw,
        None => return Err(NotFound.box_error()),
    };

    #[deriving(Encodable)]
    struct R { keyword: EncodableKeyword }
    Ok(req.json(&R { keyword: kw.encodable() }))
}

