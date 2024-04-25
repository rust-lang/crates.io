use diesel::pg::Pg;
use diesel::sql_types::Text;
use diesel::Queryable;

/// A wrapper around `semver::Version` that implements `diesel::Queryable`.
///
/// ## Example
///
/// ```rust
/// # use crates_io::sql::SemverVersion;
/// # use crates_io::schema::versions;
/// # use diesel::prelude::*;
/// #
/// #[derive(Clone, Debug, Queryable, Selectable)]
/// struct Version {
///     #[diesel(deserialize_as = SemverVersion)]
///     num: semver::Version,
/// }
/// ```
#[derive(Clone, Debug)]
pub struct SemverVersion(semver::Version);

impl From<SemverVersion> for semver::Version {
    fn from(version: SemverVersion) -> Self {
        version.0
    }
}

impl Queryable<Text, Pg> for SemverVersion {
    type Row = String;

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        row.parse().map(SemverVersion).map_err(Into::into)
    }
}
