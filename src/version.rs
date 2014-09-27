use std::collections::{HashSet, HashMap};
use time::Timespec;

use conduit::{Request, Response};
use conduit_router::RequestParams;
use pg::PostgresRow;
use pg::types::ToSql;
use semver;
use url;

use db::{Connection, RequestTransaction};
use krate::Crate;
use util::{RequestUtils, CargoResult, Require, internal, CargoError};
use util::errors::NotFound;

#[deriving(Clone)]
pub struct Version {
    pub id: i32,
    pub crate_id: i32,
    pub num: semver::Version,
    pub updated_at: Timespec,
    pub created_at: Timespec,
    pub downloads: i32,
}

#[deriving(Encodable, Decodable)]
pub struct EncodableVersion {
    pub id: i32,
    pub krate: String,
    pub num: String,
    pub dl_path: String,
    pub updated_at: String,
    pub created_at: String,
    pub downloads: i32,
}

impl Version {
    pub fn from_row(row: &PostgresRow) -> Version {
        let num: String = row.get("num");
        Version {
            id: row.get("id"),
            crate_id: row.get("crate_id"),
            num: semver::Version::parse(num.as_slice()).unwrap(),
            updated_at: row.get("updated_at"),
            created_at: row.get("created_at"),
            downloads: row.get("downloads"),
        }
    }

    pub fn find(conn: &Connection, version_id: i32)
                -> CargoResult<Version> {
        let stmt = try!(conn.prepare("SELECT * FROM versions \
                                      WHERE id = $1"));
        let mut rows = try!(stmt.query(&[&version_id]));
        match rows.next().map(|r| Version::from_row(&r)) {
            Some(version) => Ok(version),
            None => Err(NotFound.box_error()),
        }
    }

    pub fn find_by_num(conn: &Connection, crate_id: i32, num: &str)
                       -> CargoResult<Option<Version>> {
        let stmt = try!(conn.prepare("SELECT * FROM versions \
                                      WHERE crate_id = $1 AND num = $2"));
        let mut rows = try!(stmt.query(&[&crate_id, &num as &ToSql]));
        Ok(rows.next().map(|r| Version::from_row(&r)))
    }

    pub fn insert(conn: &Connection, crate_id: i32,
                  num: &str) -> CargoResult<Version> {
        let stmt = try!(conn.prepare("INSERT INTO versions \
                                      (crate_id, num, updated_at, \
                                       created_at, downloads) \
                                      VALUES ($1, $2, $3, $4, 0) \
                                      RETURNING *"));
        let now = ::now();
        let mut rows = try!(stmt.query(&[&crate_id, &num as &ToSql, &now, &now]));
        Ok(Version::from_row(&try!(rows.next().require(|| {
            internal("no version returned")
        }))))
    }

    pub fn valid(version: &str) -> bool {
        semver::Version::parse(version).is_ok()
    }

    pub fn encodable(self, krate: &Crate) -> EncodableVersion {
        let Version { id, crate_id, num, updated_at, created_at,
                      downloads } = self;
        assert_eq!(krate.id, crate_id);
        let num = num.to_string();
        EncodableVersion {
            dl_path: krate.dl_path(num.as_slice()),
            num: num,
            id: id,
            krate: krate.name.clone(),
            updated_at: ::encode_time(updated_at),
            created_at: ::encode_time(created_at),
            downloads: downloads,
        }
    }
}

pub fn index(req: &mut Request) -> CargoResult<Response> {
    let conn = try!(req.tx());

    // Extract all ids requested.
    let query = url::form_urlencoded::parse_str(req.query_string().unwrap_or(""));
    let ids = query.iter().filter_map(|&(ref a, ref b)| {
        if a.as_slice() == "ids[]" {
            from_str(b.as_slice())
        } else {
            None
        }
    }).collect::<Vec<i32>>();

    // Load all versions
    //
    // TODO: can rust-postgres do this for us?
    let mut versions = Vec::new();
    let mut set = HashSet::new();
    let query = format!("'{{{:#}}}'::int[]", ids.as_slice());
    let stmt = try!(conn.prepare(format!("SELECT * FROM versions \
                                          WHERE id = ANY({})",
                                         query).as_slice()));
    for row in try!(stmt.query(&[])) {
        let v = Version::from_row(&row);
        set.insert(v.crate_id);
        versions.push(v);
    }

    // Load all crates
    let ids = set.into_iter().collect::<Vec<i32>>();
    let query = format!("'{{{:#}}}'::int[]", ids.as_slice());
    let stmt = try!(conn.prepare(format!("SELECT * FROM crates \
                                          WHERE id = ANY({})",
                                         query).as_slice()));
    let mut map = HashMap::new();
    for row in try!(stmt.query(&[])) {
        let p = Crate::from_row(&row);
        map.insert(p.id, p);
    }

    // And respond!
    let versions = versions.into_iter().map(|v| {
        let id = v.crate_id;
        v.encodable(map.find(&id).unwrap())
    }).collect();

    #[deriving(Encodable)]
    struct R { versions: Vec<EncodableVersion> }
    Ok(req.json(&R { versions: versions }))
}

pub fn show(req: &mut Request) -> CargoResult<Response> {
    let id = &req.params()["version_id"];
    let id = from_str(id.as_slice()).unwrap_or(0);
    let conn = try!(req.tx());
    let version = try!(Version::find(&*conn, id));
    let krate = try!(Crate::find(&*conn, version.crate_id));

    #[deriving(Encodable)]
    struct R { version: EncodableVersion }
    Ok(req.json(&R { version: version.encodable(&krate) }))
}
