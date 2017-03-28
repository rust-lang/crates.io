use pg::rows::Row;
use time::Timespec;

use Model;

pub struct VersionDownload {
    pub id: i32,
    pub version_id: i32,
    pub downloads: i32,
    pub counted: i32,
    pub date: Timespec,
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct EncodableVersionDownload {
    pub id: i32,
    pub version: i32,
    pub downloads: i32,
    pub date: String,
}

impl VersionDownload {
    pub fn encodable(self) -> EncodableVersionDownload {
        let VersionDownload { id, version_id, downloads, date, .. } = self;
        EncodableVersionDownload {
            id: id,
            version: version_id,
            downloads: downloads,
            date: ::encode_time(date),
        }
    }
}

impl Model for VersionDownload {
    fn from_row(row: &Row) -> VersionDownload {
        VersionDownload {
            id: row.get("id"),
            version_id: row.get("version_id"),
            downloads: row.get("downloads"),
            counted: row.get("counted"),
            date: row.get("date"),
        }
    }

    fn table_name(_: Option<VersionDownload>) -> &'static str { "version_downloads" }
}

pub struct CrateDownload {
    pub id: i32,
    pub crate_id: i32,
    pub downloads: i32,
    pub date: Timespec,
}

impl Model for CrateDownload {
    fn from_row(row: &Row) -> CrateDownload {
        CrateDownload {
            id: row.get("id"),
            crate_id: row.get("crate_id"),
            downloads: row.get("downloads"),
            date: row.get("date"),
        }
    }

    fn table_name(_: Option<CrateDownload>) -> &'static str { "crate_downloads" }
}
