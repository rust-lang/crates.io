use chrono::NaiveDate;
use diesel;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use pg::rows::Row;

use Model;
use schema::version_downloads;
use version::Version;

#[derive(Queryable, Identifiable, Associations, Debug)]
#[belongs_to(Version)]
pub struct VersionDownload {
    pub id: i32,
    pub version_id: i32,
    pub downloads: i32,
    pub counted: i32,
    pub date: NaiveDate,
    pub processed: bool,
}

#[derive(Insertable, Debug)]
#[table_name = "version_downloads"]
struct NewVersionDownload(
    #[column_name(version_id)]
    i32
);

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct EncodableVersionDownload {
    pub id: i32,
    pub version: i32,
    pub downloads: i32,
    pub date: String,
}

impl VersionDownload {
    pub fn create_or_increment(version: i32, conn: &PgConnection) -> QueryResult<()> {
        use diesel::pg::upsert::*;
        use self::version_downloads::dsl::*;

        // We only update the counter for *today* (the default date),
        // nothing else. We have lots of other counters, but they're
        // all updated later on via the update-downloads script.
        let new_download = NewVersionDownload(version);
        let downloads_row = new_download.on_conflict(
            (version_id, date),
            do_update().set(downloads.eq(downloads + 1)),
        );
        diesel::insert(&downloads_row)
            .into(version_downloads)
            .execute(conn)?;
        Ok(())
    }

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
            processed: row.get("processed"),
        }
    }

    fn table_name(_: Option<VersionDownload>) -> &'static str {
        "version_downloads"
    }
}
