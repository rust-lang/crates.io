use crate::models::Version as FullVersion;
use crate::schema::{version_downloads, versions};
use chrono::NaiveDate;
use crates_io_diesel_helpers::SemverVersion;
use diesel::prelude::*;

#[derive(Queryable, Identifiable, Associations, Debug, Clone, Copy)]
#[diesel(
    primary_key(version_id, date),
    belongs_to(FullVersion, foreign_key=version_id),
    belongs_to(Version),
)]
pub struct VersionDownload {
    pub version_id: i32,
    pub downloads: i32,
    pub counted: i32,
    pub date: NaiveDate,
    pub processed: bool,
}

/// A subset of the columns of the `versions` table.
///
/// This struct is used to load all versions of a crate from the database,
/// without loading the additional data that is unnecessary for download version resolution.
///
#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = versions)]
pub struct Version {
    pub id: i32,
    #[diesel(deserialize_as = SemverVersion)]
    pub num: semver::Version,
}
