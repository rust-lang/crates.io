use std::collections::HashMap;

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::util::errors::{cargo_err, AppResult};

use crate::models::{Crate, Dependency, User, VersionOwnerAction};
use crate::schema::*;
use crate::views::{EncodableAuditAction, EncodableVersion, EncodableVersionLinks};

// Queryable has a custom implementation below
#[derive(Clone, Identifiable, Associations, Debug, Queryable, Deserialize, Serialize)]
#[belongs_to(Crate)]
pub struct Version {
    pub id: i32,
    pub crate_id: i32,
    pub num: semver::Version,
    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub downloads: i32,
    pub features: serde_json::Value,
    pub yanked: bool,
    pub license: Option<String>,
    pub crate_size: Option<i32>,
    pub published_by: Option<i32>,
}

#[derive(Insertable, Debug)]
#[table_name = "versions"]
pub struct NewVersion {
    crate_id: i32,
    num: String,
    features: serde_json::Value,
    license: Option<String>,
    crate_size: Option<i32>,
    published_by: i32,
}

/// The highest version (semver order) and the most recently updated version.
/// Typically used for a single crate.
#[derive(Debug, Clone)]
pub struct TopVersions {
    pub highest: semver::Version,
    pub newest: semver::Version,
}

/// A default semver value, "0.0.0", for use in TopVersions
fn default_semver_version() -> semver::Version {
    semver::Version {
        major: 0,
        minor: 0,
        patch: 0,
        pre: vec![],
        build: vec![],
    }
}

impl Version {
    pub fn encodable(
        self,
        crate_name: &str,
        published_by: Option<User>,
        audit_actions: Vec<(VersionOwnerAction, User)>,
    ) -> EncodableVersion {
        let Version {
            id,
            num,
            updated_at,
            created_at,
            downloads,
            features,
            yanked,
            license,
            crate_size,
            ..
        } = self;
        let num = num.to_string();
        EncodableVersion {
            dl_path: format!("/api/v1/crates/{}/{}/download", crate_name, num),
            readme_path: format!("/api/v1/crates/{}/{}/readme", crate_name, num),
            num: num.clone(),
            id,
            krate: crate_name.to_string(),
            updated_at,
            created_at,
            downloads,
            features,
            yanked,
            license,
            links: EncodableVersionLinks {
                dependencies: format!("/api/v1/crates/{}/{}/dependencies", crate_name, num),
                version_downloads: format!("/api/v1/crates/{}/{}/downloads", crate_name, num),
                authors: format!("/api/v1/crates/{}/{}/authors", crate_name, num),
            },
            crate_size,
            published_by: published_by.map(User::encodable_public),
            audit_actions: audit_actions
                .into_iter()
                .map(|(audit_action, user)| EncodableAuditAction {
                    action: audit_action.action.into(),
                    user: User::encodable_public(user),
                    time: audit_action.time,
                })
                .collect(),
        }
    }

    /// Returns (dependency, crate dependency name)
    pub fn dependencies(&self, conn: &PgConnection) -> QueryResult<Vec<(Dependency, String)>> {
        Dependency::belonging_to(self)
            .inner_join(crates::table)
            .select((dependencies::all_columns, crates::name))
            .order((dependencies::optional, crates::name))
            .load(conn)
    }

    /// Return both the newest (most recently updated) and the
    /// highest version (in semver order) for a collection of date/version pairs.
    pub fn top<T>(pairs: T) -> TopVersions
    where
        T: Clone + IntoIterator<Item = (NaiveDateTime, semver::Version)>,
    {
        TopVersions {
            newest: pairs
                .clone()
                .into_iter()
                .max()
                .unwrap_or((
                    NaiveDateTime::from_timestamp(0, 0),
                    default_semver_version(),
                ))
                .1,
            highest: pairs
                .into_iter()
                .map(|(_, v)| v)
                .max()
                .unwrap_or_else(default_semver_version),
        }
    }

    pub fn record_readme_rendering(version_id_: i32, conn: &PgConnection) -> QueryResult<usize> {
        use crate::schema::readme_renderings::dsl::*;
        use diesel::dsl::now;

        diesel::insert_into(readme_renderings)
            .values(version_id.eq(version_id_))
            .on_conflict(version_id)
            .do_update()
            .set(rendered_at.eq(now))
            .execute(conn)
    }

    /// Gets the User who ran `cargo publish` for this version, if recorded.
    /// Not for use when you have a group of versions you need the publishers for.
    pub fn published_by(&self, conn: &PgConnection) -> Option<User> {
        match self.published_by {
            Some(pb) => users::table.find(pb).first(conn).ok(),
            None => None,
        }
    }
}

impl NewVersion {
    pub fn new(
        crate_id: i32,
        num: &semver::Version,
        features: &HashMap<String, Vec<String>>,
        license: Option<String>,
        license_file: Option<&str>,
        crate_size: i32,
        published_by: i32,
    ) -> AppResult<Self> {
        let features = serde_json::to_value(features)?;

        let mut new_version = NewVersion {
            crate_id,
            num: num.to_string(),
            features,
            license,
            crate_size: Some(crate_size),
            published_by,
        };

        new_version.validate_license(license_file)?;

        Ok(new_version)
    }

    pub fn save(
        &self,
        conn: &PgConnection,
        authors: &[String],
        published_by_email: &str,
    ) -> AppResult<Version> {
        use crate::schema::version_authors::{name, version_id};
        use crate::schema::versions::dsl::*;
        use diesel::dsl::exists;
        use diesel::{insert_into, select};

        conn.transaction(|| {
            let already_uploaded = versions
                .filter(crate_id.eq(self.crate_id))
                .filter(num.eq(&self.num));
            if select(exists(already_uploaded)).get_result(conn)? {
                return Err(cargo_err(&format_args!(
                    "crate version `{}` is already \
                     uploaded",
                    self.num
                )));
            }

            let version = insert_into(versions)
                .values(self)
                .get_result::<Version>(conn)?;

            insert_into(versions_published_by::table)
                .values((
                    versions_published_by::version_id.eq(version.id),
                    versions_published_by::email.eq(published_by_email),
                ))
                .execute(conn)?;

            let new_authors = authors
                .iter()
                .map(|s| (version_id.eq(version.id), name.eq(s)))
                .collect::<Vec<_>>();

            insert_into(version_authors::table)
                .values(&new_authors)
                .execute(conn)?;
            Ok(version)
        })
    }

    fn validate_license(&mut self, license_file: Option<&str>) -> AppResult<()> {
        if let Some(ref license) = self.license {
            for part in license.split('/') {
                license_exprs::validate_license_expr(part).map_err(|e| {
                    cargo_err(&format_args!(
                        "{}; see http://opensource.org/licenses \
                         for options, and http://spdx.org/licenses/ \
                         for their identifiers",
                        e
                    ))
                })?;
            }
        } else if license_file.is_some() {
            // If no license is given, but a license file is given, flag this
            // crate as having a nonstandard license. Note that we don't
            // actually do anything else with license_file currently.
            self.license = Some(String::from("non-standard"));
        }
        Ok(())
    }
}
