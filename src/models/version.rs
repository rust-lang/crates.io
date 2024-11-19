use std::collections::BTreeMap;

use bon::Builder;
use chrono::NaiveDateTime;
use crates_io_index::features::FeaturesMap;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use serde::Deserialize;

use crate::models::{Crate, User};
use crate::schema::*;

// Queryable has a custom implementation below
#[derive(Clone, Identifiable, Associations, Debug, Queryable, Selectable)]
#[diesel(belongs_to(Crate))]
pub struct Version {
    pub id: i32,
    pub crate_id: i32,
    pub num: String,
    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub downloads: i32,
    pub features: serde_json::Value,
    pub yanked: bool,
    pub license: Option<String>,
    pub crate_size: i32,
    pub published_by: Option<i32>,
    pub checksum: String,
    pub links: Option<String>,
    pub rust_version: Option<String>,
    pub has_lib: Option<bool>,
    pub bin_names: Option<Vec<Option<String>>>,
    pub yank_message: Option<String>,
    pub num_no_build: String,
    pub edition: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
}

impl Version {
    pub async fn record_readme_rendering(
        version_id: i32,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<usize> {
        use diesel::dsl::now;

        diesel::insert_into(readme_renderings::table)
            .values(readme_renderings::version_id.eq(version_id))
            .on_conflict(readme_renderings::version_id)
            .do_update()
            .set(readme_renderings::rendered_at.eq(now))
            .execute(conn)
            .await
    }

    /// Gets the User who ran `cargo publish` for this version, if recorded.
    /// Not for use when you have a group of versions you need the publishers for.
    pub async fn published_by(&self, conn: &mut AsyncPgConnection) -> QueryResult<Option<User>> {
        match self.published_by {
            Some(pb) => users::table.find(pb).first(conn).await.optional(),
            None => Ok(None),
        }
    }

    /// Deserializes the `features` field from JSON into a `BTreeMap`.
    ///
    /// # Returns
    ///
    /// * `Ok(BTreeMap<String, Vec<String>>)` - If the deserialization was successful.
    /// * `Err(serde_json::Error)` - If the deserialization failed.
    pub fn features(&self) -> Result<FeaturesMap, serde_json::Error> {
        BTreeMap::<String, Vec<String>>::deserialize(&self.features)
    }
}

#[derive(Insertable, Debug, Builder)]
#[diesel(table_name = versions, check_for_backend(diesel::pg::Pg))]
pub struct NewVersion<'a> {
    #[builder(start_fn)]
    crate_id: i32,
    #[builder(start_fn)]
    num: &'a str,
    #[builder(default = strip_build_metadata(num))]
    pub num_no_build: &'a str,
    created_at: Option<&'a NaiveDateTime>,
    yanked: Option<bool>,
    #[builder(default = serde_json::Value::Object(Default::default()))]
    features: serde_json::Value,
    license: Option<&'a str>,
    #[builder(default, name = "size")]
    crate_size: i32,
    published_by: i32,
    checksum: &'a str,
    links: Option<&'a str>,
    rust_version: Option<&'a str>,
    pub has_lib: Option<bool>,
    pub bin_names: Option<&'a [&'a str]>,
    edition: Option<&'a str>,
}

impl NewVersion<'_> {
    pub async fn save(
        &self,
        conn: &mut AsyncPgConnection,
        published_by_email: &str,
    ) -> QueryResult<Version> {
        use diesel::insert_into;

        conn.transaction(|conn| {
            async move {
                let version: Version = insert_into(versions::table)
                    .values(self)
                    .get_result(conn)
                    .await?;

                insert_into(versions_published_by::table)
                    .values((
                        versions_published_by::version_id.eq(version.id),
                        versions_published_by::email.eq(published_by_email),
                    ))
                    .execute(conn)
                    .await?;

                Ok(version)
            }
            .scope_boxed()
        })
        .await
    }
}

fn strip_build_metadata(version: &str) -> &str {
    version
        .split_once('+')
        .map(|parts| parts.0)
        .unwrap_or(version)
}

/// The highest version (semver order) and the most recently updated version.
/// Typically used for a single crate.
/// Note: `TopVersion` itself does not guarantee whether versions are yanked or not,
/// this must be guaranteed by the input versions.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TopVersions {
    /// The "highest" version in terms of semver
    pub highest: Option<semver::Version>,
    /// The "highest" non-prerelease version
    pub highest_stable: Option<semver::Version>,
    /// The "newest" version in terms of publishing date
    pub newest: Option<semver::Version>,
}

impl TopVersions {
    /// Return both the newest (most recently updated) and the
    /// highest version (in semver order) for a list of `Version` instances.
    pub fn from_versions(versions: Vec<Version>) -> Self {
        Self::from_date_version_pairs(versions.into_iter().map(|v| (v.created_at, v.num)))
    }

    /// Return both the newest (most recently updated) and the
    /// highest version (in semver order) for a collection of date/version pairs.
    pub fn from_date_version_pairs<T>(pairs: T) -> Self
    where
        T: IntoIterator<Item = (NaiveDateTime, String)>,
    {
        // filter out versions that we can't parse
        let pairs: Vec<(NaiveDateTime, semver::Version)> = pairs
            .into_iter()
            .filter_map(|(date, version)| {
                semver::Version::parse(&version)
                    .ok()
                    .map(|version| (date, version))
            })
            .collect();

        let newest = pairs.iter().max().map(|(_, v)| v.clone());
        let highest = pairs.iter().map(|(_, v)| v).max().cloned();
        let highest_stable = pairs
            .iter()
            .map(|(_, v)| v)
            .filter(|v| v.pre.is_empty())
            .max()
            .cloned();

        Self {
            highest,
            highest_stable,
            newest,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TopVersions;
    use chrono::NaiveDateTime;

    #[track_caller]
    fn date(str: &str) -> NaiveDateTime {
        str.parse().unwrap()
    }

    #[track_caller]
    fn version(str: &str) -> semver::Version {
        semver::Version::parse(str).unwrap()
    }

    #[test]
    fn top_versions_empty() {
        let versions = vec![];
        assert_eq!(
            TopVersions::from_date_version_pairs(versions),
            TopVersions {
                highest: None,
                highest_stable: None,
                newest: None,
            }
        );
    }

    #[test]
    fn top_versions_single() {
        let versions = vec![(date("2020-12-03T12:34:56"), "1.0.0".into())];
        assert_eq!(
            TopVersions::from_date_version_pairs(versions),
            TopVersions {
                highest: Some(version("1.0.0")),
                highest_stable: Some(version("1.0.0")),
                newest: Some(version("1.0.0")),
            }
        );
    }

    #[test]
    fn top_versions_prerelease() {
        let versions = vec![(date("2020-12-03T12:34:56"), "1.0.0-beta.5".into())];
        assert_eq!(
            TopVersions::from_date_version_pairs(versions),
            TopVersions {
                highest: Some(version("1.0.0-beta.5")),
                highest_stable: None,
                newest: Some(version("1.0.0-beta.5")),
            }
        );
    }

    #[test]
    fn top_versions_multiple() {
        let versions = vec![
            (date("2018-12-03T12:34:56"), "1.0.0".into()),
            (date("2019-12-03T12:34:56"), "2.0.0-alpha.1".into()),
            (date("2020-12-01T12:34:56"), "everything is broken".into()),
            (date("2020-12-03T12:34:56"), "1.1.0".into()),
            (date("2020-12-31T12:34:56"), "1.0.4".into()),
        ];
        assert_eq!(
            TopVersions::from_date_version_pairs(versions),
            TopVersions {
                highest: Some(version("2.0.0-alpha.1")),
                highest_stable: Some(version("1.1.0")),
                newest: Some(version("1.0.4")),
            }
        );
    }
}
