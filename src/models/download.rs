use chrono::NaiveDate;
use diesel::prelude::*;

use crate::models::Version;
use crate::schema::version_downloads;
use crate::views::EncodableVersionDownload;

#[derive(Queryable, Identifiable, Associations, Debug, Clone, Copy)]
#[belongs_to(Version)]
#[primary_key(version_id, date)]
pub struct VersionDownload {
    pub version_id: i32,
    pub downloads: i32,
    pub counted: i32,
    pub date: NaiveDate,
    pub processed: bool,
}

impl VersionDownload {
    pub fn create_or_increment(version: i32, conn: &PgConnection) -> QueryResult<()> {
        use self::version_downloads::dsl::*;

        // We only update the counter for *today* (the default date),
        // nothing else. We have lots of other counters, but they're
        // all updated later on via the update-downloads script.
        diesel::insert_into(version_downloads)
            .values(version_id.eq(version))
            .on_conflict((version_id, date))
            .do_update()
            .set(downloads.eq(downloads + 1))
            .execute(conn)?;
        Ok(())
    }

    pub fn encodable(self) -> EncodableVersionDownload {
        EncodableVersionDownload {
            version: self.version_id,
            downloads: self.downloads,
            date: self.date.to_string(),
        }
    }
}
