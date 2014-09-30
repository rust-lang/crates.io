use time::Timespec;
use pg::PostgresRow;

use db::Connection;
use util::{CargoResult, CargoError};
use util::errors::NotFound;

pub struct VersionDownload {
    pub id: i32,
    pub version_id: i32,
    pub downloads: i32,
    pub counted: i32,
    pub date: Timespec,
}

impl VersionDownload {
    pub fn from_row(row: &PostgresRow) -> VersionDownload {
        VersionDownload {
            id: row.get("id"),
            version_id: row.get("version_id"),
            downloads: row.get("downloads"),
            counted: row.get("counted"),
            date: row.get("date"),
        }
    }

    pub fn find(conn: &Connection, id: i32)
                -> CargoResult<VersionDownload> {
        let stmt = try!(conn.prepare("SELECT * FROM version_downloads \
                                      WHERE id = $1"));
        let mut rows = try!(stmt.query(&[&id]));
        match rows.next().map(|r| VersionDownload::from_row(&r)) {
            Some(version) => Ok(version),
            None => Err(NotFound.box_error()),
        }
    }
}

pub struct CrateDownload {
    pub id: i32,
    pub crate_id: i32,
    pub downloads: i32,
    pub date: Timespec,
}

impl CrateDownload {
    pub fn from_row(row: &PostgresRow) -> CrateDownload {
        CrateDownload {
            id: row.get("id"),
            crate_id: row.get("crate_id"),
            downloads: row.get("downloads"),
            date: row.get("date"),
        }
    }

    pub fn find(conn: &Connection, id: i32)
                -> CargoResult<CrateDownload> {
        let stmt = try!(conn.prepare("SELECT * FROM crate_downloads \
                                      WHERE id = $1"));
        let mut rows = try!(stmt.query(&[&id]));
        match rows.next().map(|r| CrateDownload::from_row(&r)) {
            Some(version) => Ok(version),
            None => Err(NotFound.box_error()),
        }
    }
}
