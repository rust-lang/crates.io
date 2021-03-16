use crate::models::Version;
use crate::schema::version_downloads;
use chrono::NaiveDate;

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
