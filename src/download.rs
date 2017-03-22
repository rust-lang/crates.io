use chrono::NaiveDate;
use pg::rows::Row;

use Model;

pub struct VersionDownload {
    pub id: i32,
    pub version_id: i32,
    pub downloads: i32,
    pub counted: i32,
    pub date: NaiveDate,
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
        EncodableVersionDownload {
            id: self.id,
            version: self.version_id,
            downloads: self.downloads,
            date: self.date.to_string(),
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
