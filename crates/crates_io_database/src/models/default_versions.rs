use crate::schema::{default_versions, versions};
use crates_io_diesel_helpers::SemverVersion;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use tracing::{debug, instrument, warn};

/// A subset of the columns of the `versions` table.
///
/// This struct is used to load all versions of a crate from the database,
/// without loading all the additional data unnecessary for default version
/// resolution.
///
/// It implements [Ord] in a way that sorts versions by the criteria specified
/// in the [update_default_version] function documentation. The default version
/// will be the "maximum" element in a sorted list of versions.
#[derive(Clone, Debug, PartialEq, Eq, Queryable, Selectable)]
#[diesel(table_name = versions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Version {
    pub id: i32,
    #[diesel(deserialize_as = SemverVersion)]
    pub num: semver::Version,
    pub yanked: bool,
}

impl Version {
    /// Returns `true` if the version contains a pre-release identifier.
    fn is_prerelease(&self) -> bool {
        !self.num.pre.is_empty()
    }

    fn ord_tuple(&self) -> (bool, bool, &semver::Version, i32) {
        (!self.yanked, !self.is_prerelease(), &self.num, self.id)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ord_tuple().cmp(&other.ord_tuple())
    }
}

/// Updates the `default_versions` table entry for the specified crate.
///
/// This function first loads all versions of the crate from the database,
/// then determines the default version based on the following criteria:
///
/// 1. The highest non-prerelease version that is not yanked.
/// 2. The highest non-yanked version.
/// 3. The highest version.
///
/// The default version is then written to the `default_versions` table.
#[instrument(skip(conn))]
pub async fn update_default_version(
    crate_id: i32,
    conn: &mut AsyncPgConnection,
) -> QueryResult<()> {
    let default_version = calculate_default_version(crate_id, conn).await?;

    debug!(
        "Updating default version to {} (id: {})…",
        default_version.num, default_version.id
    );

    diesel::insert_into(default_versions::table)
        .values((
            default_versions::crate_id.eq(crate_id),
            default_versions::version_id.eq(default_version.id),
        ))
        .on_conflict(default_versions::crate_id)
        .do_update()
        .set(default_versions::version_id.eq(default_version.id))
        .execute(conn)
        .await?;

    Ok(())
}

/// Verifies that the default version for the specified crate is up-to-date.
#[instrument(skip(conn))]
pub async fn verify_default_version(
    crate_id: i32,
    conn: &mut AsyncPgConnection,
) -> QueryResult<()> {
    let calculated = calculate_default_version(crate_id, conn).await?;

    let saved = default_versions::table
        .select(default_versions::version_id)
        .filter(default_versions::crate_id.eq(crate_id))
        .first::<i32>(conn)
        .await
        .optional()?;

    if let Some(saved) = saved {
        if saved == calculated.id {
            debug!("Default version for crate {crate_id} is up to date");
        } else {
            warn!(
                "Default version for crate {crate_id} is outdated (expected: {saved}, actual: {})",
                calculated.id,
            );
        }
    } else {
        warn!(
            "Default version for crate {crate_id} is missing (expected: {})",
            calculated.id
        );
    }

    Ok(())
}

async fn calculate_default_version(
    crate_id: i32,
    conn: &mut AsyncPgConnection,
) -> QueryResult<Version> {
    use diesel::result::Error::NotFound;

    debug!("Loading all versions for the crate…");
    let versions = versions::table
        .filter(versions::crate_id.eq(crate_id))
        .select(Version::as_returning())
        .load::<Version>(conn)
        .await?;

    debug!("Found {} versions", versions.len());

    versions.into_iter().max().ok_or(NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::crates;
    use claims::assert_some;
    use crates_io_test_db::TestDatabase;
    use insta::assert_snapshot;
    use std::fmt::Write;

    fn v(num: &str, yanked: bool) -> Version {
        let num = semver::Version::parse(num).unwrap();
        Version { id: 0, num, yanked }
    }

    #[test]
    fn test_find_default_version() {
        fn check(versions: &[Version], expected: &str) {
            let default_version = assert_some!(versions.iter().max());
            assert_eq!(default_version.num.to_string(), expected);
        }

        // Only a single version
        let versions = vec![v("1.0.0", false)];
        check(&versions, "1.0.0");

        // Multiple versions out of order
        let versions = vec![
            v("1.0.0", false),
            v("1.0.1", false),
            v("1.1.0", false),
            v("1.0.2", false),
        ];
        check(&versions, "1.1.0");

        // Multiple versions with one pre-release
        let versions = vec![
            v("1.0.0", false),
            v("1.1.0", false),
            v("2.0.0-beta.1", false),
        ];
        check(&versions, "1.1.0");

        // Only pre-release versions
        let versions = vec![
            v("1.0.0-beta.1", false),
            v("1.0.0-beta.2", false),
            v("1.0.0-beta.3", false),
        ];
        check(&versions, "1.0.0-beta.3");

        // Only pre-release versions, with highest yanked
        let versions = vec![
            v("1.0.0-beta.1", false),
            v("1.0.0-beta.2", false),
            v("1.0.0-beta.3", true),
        ];
        check(&versions, "1.0.0-beta.2");

        // Only yanked versions
        let versions = vec![
            v("1.0.0-beta.1", true),
            v("1.0.0-beta.2", true),
            v("1.0.0-beta.3", true),
        ];
        check(&versions, "1.0.0-beta.3");
    }

    #[test]
    fn test_ord() {
        let mut versions = vec![
            v("1.0.0", false),
            v("1.0.0-beta.1", false),
            v("1.0.0-beta.2", false),
            v("1.0.0-beta.3", false),
            v("1.0.1", true),
            v("1.0.2", false),
            v("1.1.0", false),
            v("1.1.1-beta.1", true),
            v("1.1.1", true),
            v("1.0.3", false),
            v("2.0.0-beta.1", false),
        ];

        versions.sort();

        assert_snapshot!(format_versions(&versions), @r"
        1.1.1-beta.1 (yanked)
        1.0.1 (yanked)
        1.1.1 (yanked)
        1.0.0-beta.1
        1.0.0-beta.2
        1.0.0-beta.3
        2.0.0-beta.1
        1.0.0
        1.0.2
        1.0.3
        1.1.0
        ");
    }

    fn format_versions(versions: &[Version]) -> String {
        let mut buf = String::with_capacity(versions.len() * 20);
        for v in versions {
            write!(buf, "{}", v.num).unwrap();
            if v.yanked {
                buf.push_str(" (yanked)");
            }
            buf.push('\n');
        }
        buf
    }

    async fn create_crate(name: &str, conn: &mut AsyncPgConnection) -> i32 {
        diesel::insert_into(crates::table)
            .values(crates::name.eq(name))
            .returning(crates::id)
            .get_result(conn)
            .await
            .unwrap()
    }

    async fn create_version(crate_id: i32, num: &str, conn: &mut AsyncPgConnection) {
        diesel::insert_into(versions::table)
            .values((
                versions::crate_id.eq(crate_id),
                versions::num.eq(num),
                versions::num_no_build.eq(num),
                versions::checksum.eq(""),
                versions::crate_size.eq(0),
            ))
            .execute(conn)
            .await
            .unwrap();
    }

    async fn get_default_version(crate_id: i32, conn: &mut AsyncPgConnection) -> String {
        default_versions::table
            .inner_join(versions::table)
            .select(versions::num)
            .filter(default_versions::crate_id.eq(crate_id))
            .first(conn)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_update_default_version() {
        let test_db = TestDatabase::new();
        let conn = &mut test_db.async_connect().await;

        let crate_id = create_crate("foo", conn).await;
        create_version(crate_id, "1.0.0", conn).await;

        update_default_version(crate_id, conn).await.unwrap();
        assert_eq!(get_default_version(crate_id, conn).await, "1.0.0");

        create_version(crate_id, "1.1.0", conn).await;
        create_version(crate_id, "1.0.1", conn).await;
        assert_eq!(get_default_version(crate_id, conn).await, "1.0.0");

        update_default_version(crate_id, conn).await.unwrap();
        assert_eq!(get_default_version(crate_id, conn).await, "1.1.0");
    }
}
