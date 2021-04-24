use cargo_registry::{
    models::{Crate, NewVersion, Version},
    schema::{dependencies, versions},
    util::errors::AppResult,
};
use std::collections::HashMap;

use chrono::NaiveDateTime;
use diesel::prelude::*;

/// A builder to create version records for the purpose of inserting directly into the database.
pub struct VersionBuilder<'a> {
    created_at: Option<NaiveDateTime>,
    dependencies: Vec<(i32, Option<&'static str>)>,
    features: HashMap<String, Vec<String>>,
    license: Option<&'a str>,
    license_file: Option<&'a str>,
    num: semver::Version,
    size: i32,
    yanked: bool,
}

impl<'a> VersionBuilder<'a> {
    /// Creates a VersionBuilder from a string slice `num` representing the version's number.
    ///
    /// # Panics
    ///
    /// Panics if `num` cannot be parsed as a valid `semver::Version`.
    #[track_caller]
    pub fn new(num: &str) -> Self {
        let num = semver::Version::parse(num).unwrap_or_else(|e| {
            panic!("The version {} is not valid: {}", num, e);
        });

        VersionBuilder {
            created_at: None,
            dependencies: Vec::new(),
            features: HashMap::new(),
            license: None,
            license_file: None,
            num,
            size: 0,
            yanked: false,
        }
    }

    /// Sets the version's `created_at` value.
    pub fn created_at(mut self, created_at: NaiveDateTime) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Sets the version's `license` value.
    pub fn license(mut self, license: Option<&'a str>) -> Self {
        self.license = license;
        self
    }

    /// Adds a dependency to this version.
    pub fn dependency(mut self, dependency: &Crate, target: Option<&'static str>) -> Self {
        self.dependencies.push((dependency.id, target));
        self
    }

    /// Sets the version's `yanked` value.
    pub fn yanked(self, yanked: bool) -> Self {
        Self { yanked, ..self }
    }

    /// Sets the version's size.
    pub fn size(mut self, size: i32) -> Self {
        self.size = size;
        self
    }

    pub fn build(
        self,
        crate_id: i32,
        published_by: i32,
        connection: &PgConnection,
    ) -> AppResult<Version> {
        use diesel::{insert_into, update};

        let license = self.license.map(|license| license.to_owned());

        let mut vers = NewVersion::new(
            crate_id,
            &self.num,
            &self.features,
            license,
            self.license_file,
            self.size,
            published_by,
        )?
        .save(connection, "someone@example.com")?;

        if self.yanked {
            vers = update(&vers)
                .set(versions::yanked.eq(true))
                .get_result(connection)?;
        }

        if let Some(created_at) = self.created_at {
            vers = update(&vers)
                .set(versions::created_at.eq(created_at))
                .get_result(connection)?;
        }

        let new_deps = self
            .dependencies
            .into_iter()
            .map(|(crate_id, target)| {
                (
                    dependencies::version_id.eq(vers.id),
                    dependencies::req.eq(">= 0"),
                    dependencies::crate_id.eq(crate_id),
                    dependencies::target.eq(target),
                    dependencies::optional.eq(false),
                    dependencies::default_features.eq(false),
                    dependencies::features.eq(Vec::<String>::new()),
                )
            })
            .collect::<Vec<_>>();
        insert_into(dependencies::table)
            .values(&new_deps)
            .execute(connection)?;

        Ok(vers)
    }

    /// Consumes the builder and creates the version record in the database.
    ///
    /// # Panics
    ///
    /// Panics (and fails the test) if any part of inserting the version record fails.
    #[track_caller]
    pub fn expect_build(
        self,
        crate_id: i32,
        published_by: i32,
        connection: &PgConnection,
    ) -> Version {
        self.build(crate_id, published_by, connection)
            .unwrap_or_else(|e| {
                panic!("Unable to create version: {:?}", e);
            })
    }
}

impl<'a> From<&'a str> for VersionBuilder<'a> {
    fn from(num: &'a str) -> Self {
        VersionBuilder::new(num)
    }
}
