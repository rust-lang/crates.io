use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use derive_builder::Builder;
use diesel::prelude::*;

use crate::util::errors::{bad_request, AppResult};

use crate::models::{Crate, Dependency, User};
use crate::schema::*;
use crate::sql::split_part;
use crate::util::diesel::Conn;

// Queryable has a custom implementation below
#[derive(Clone, Identifiable, Associations, Debug, Queryable)]
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
    pub crate_size: Option<i32>,
    pub published_by: Option<i32>,
    pub checksum: String,
    pub links: Option<String>,
    pub rust_version: Option<String>,
    pub has_lib: Option<bool>,
    pub bin_names: Option<Vec<Option<String>>>,
}

impl Version {
    /// Returns (dependency, crate dependency name)
    pub fn dependencies(&self, conn: &mut impl Conn) -> QueryResult<Vec<(Dependency, String)>> {
        Dependency::belonging_to(self)
            .inner_join(crates::table)
            .select((dependencies::all_columns, crates::name))
            .order((dependencies::optional, crates::name))
            .load(conn)
    }

    pub fn record_readme_rendering(version_id: i32, conn: &mut impl Conn) -> QueryResult<usize> {
        use diesel::dsl::now;

        diesel::insert_into(readme_renderings::table)
            .values(readme_renderings::version_id.eq(version_id))
            .on_conflict(readme_renderings::version_id)
            .do_update()
            .set(readme_renderings::rendered_at.eq(now))
            .execute(conn)
    }

    /// Gets the User who ran `cargo publish` for this version, if recorded.
    /// Not for use when you have a group of versions you need the publishers for.
    pub fn published_by(&self, conn: &mut impl Conn) -> Option<User> {
        match self.published_by {
            Some(pb) => users::table.find(pb).first(conn).ok(),
            None => None,
        }
    }
}

#[derive(Insertable, Debug, Builder)]
#[diesel(table_name = versions, check_for_backend(diesel::pg::Pg))]
pub struct NewVersion {
    crate_id: i32,
    num: String,
    #[builder(
        default = "serde_json::Value::Object(Default::default())",
        setter(custom)
    )]
    features: serde_json::Value,
    #[builder(default)]
    license: Option<String>,
    #[builder(default, setter(name = "size"))]
    crate_size: i32,
    published_by: i32,
    #[builder(setter(into))]
    checksum: String,
    #[builder(default)]
    links: Option<String>,
    #[builder(default)]
    rust_version: Option<String>,
    #[builder(default, setter(strip_option))]
    pub has_lib: Option<bool>,
    #[builder(default, setter(strip_option))]
    pub bin_names: Option<Vec<String>>,
}

impl NewVersionBuilder {
    pub fn features(
        &mut self,
        features: &BTreeMap<String, Vec<String>>,
    ) -> serde_json::Result<&mut Self> {
        self.features = Some(serde_json::to_value(features)?);
        Ok(self)
    }

    /// Set the `checksum` field to a basic dummy value.
    pub fn dummy_checksum(&mut self) -> &mut Self {
        const DUMMY_CHECKSUM: &str =
            "0000000000000000000000000000000000000000000000000000000000000000";

        self.checksum = Some(DUMMY_CHECKSUM.to_string());
        self
    }
}

impl NewVersion {
    pub fn builder(crate_id: i32, version: impl Into<String>) -> NewVersionBuilder {
        let mut builder = NewVersionBuilder::default();
        builder.crate_id(crate_id).num(version.into());
        builder
    }

    pub fn save(&self, conn: &mut impl Conn, published_by_email: &str) -> AppResult<Version> {
        use diesel::dsl::exists;
        use diesel::{insert_into, select};

        conn.transaction(|conn| {
            let num_no_build = strip_build_metadata(&self.num);

            let already_uploaded = versions::table
                .filter(versions::crate_id.eq(self.crate_id))
                .filter(split_part(versions::num, "+", 1).eq(num_no_build));

            if select(exists(already_uploaded)).get_result(conn)? {
                return Err(bad_request(format_args!(
                    "crate version `{}` is already uploaded",
                    num_no_build
                )));
            }

            let version: Version = insert_into(versions::table).values(self).get_result(conn)?;

            insert_into(versions_published_by::table)
                .values((
                    versions_published_by::version_id.eq(version.id),
                    versions_published_by::email.eq(published_by_email),
                ))
                .execute(conn)?;
            Ok(version)
        })
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
