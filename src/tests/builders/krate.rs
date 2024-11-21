use crate::{
    models::{Category, Crate, Keyword, NewCrate},
    schema::{crates, version_downloads},
    util::errors::AppResult,
};

use super::VersionBuilder;
use crate::models::update_default_version;
use crate::schema::crate_downloads;
use crate::util::diesel::prelude::*;
use chrono::NaiveDateTime;
use diesel_async::AsyncPgConnection;

/// A builder to create crate records for the purpose of inserting directly into the database.
/// If you want to test logic that happens as part of a publish request, use `PublishBuilder`
/// instead.
pub struct CrateBuilder<'a> {
    categories: Vec<&'a str>,
    downloads: Option<i32>,
    keywords: Vec<&'a str>,
    krate: NewCrate<'a>,
    owner_id: i32,
    recent_downloads: Option<i32>,
    updated_at: Option<NaiveDateTime>,
    versions: Vec<VersionBuilder>,
}

impl<'a> CrateBuilder<'a> {
    /// Create a new instance with the given crate name and owner. If the owner with the given ID
    /// doesn't exist in the database, `expect_build` will fail.
    pub fn new(name: &str, owner_id: i32) -> CrateBuilder<'_> {
        CrateBuilder {
            categories: Vec::new(),
            downloads: None,
            keywords: Vec::new(),
            krate: NewCrate {
                name,
                ..NewCrate::default()
            },
            owner_id,
            recent_downloads: None,
            updated_at: None,
            versions: Vec::new(),
        }
    }

    /// Sets the crate's `description` value.
    pub fn description(mut self, description: &'a str) -> Self {
        self.krate.description = Some(description);
        self
    }

    /// Sets the crate's `documentation` URL.
    pub fn documentation(mut self, documentation: &'a str) -> Self {
        self.krate.documentation = Some(documentation);
        self
    }

    /// Sets the crate's `homepage` URL.
    pub fn homepage(mut self, homepage: &'a str) -> Self {
        self.krate.homepage = Some(homepage);
        self
    }

    /// Sets the crate's `readme` content.
    pub fn readme(mut self, readme: &'a str) -> Self {
        self.krate.readme = Some(readme);
        self
    }

    /// Sets the crate's `max_upload_size` override value.
    pub fn max_upload_size(mut self, max_upload_size: i32) -> Self {
        self.krate.max_upload_size = Some(max_upload_size);
        self
    }

    /// Sets the crate's number of downloads that happened more than 90 days ago. The total
    /// number of downloads for this crate will be this plus the number of recent downloads.
    pub fn downloads(mut self, downloads: i32) -> Self {
        self.downloads = Some(downloads);
        self
    }

    /// Sets the crate's number of downloads in the last 90 days. The total number of downloads
    /// for this crate will be this plus the number of downloads set with the `downloads` method.
    pub fn recent_downloads(mut self, recent_downloads: i32) -> Self {
        self.recent_downloads = Some(recent_downloads);
        self
    }

    /// Adds a version record to be associated with the crate record when the crate record is
    /// built.
    pub fn version<T: Into<VersionBuilder>>(mut self, version: T) -> Self {
        self.versions.push(version.into());
        self
    }

    /// Adds a category to the crate.
    pub fn category(mut self, category: &'a str) -> Self {
        self.categories.push(category);
        self
    }

    /// Adds a keyword to the crate.
    pub fn keyword(mut self, keyword: &'a str) -> Self {
        self.keywords.push(keyword);
        self
    }

    /// Sets the crate's `updated_at` value.
    pub fn updated_at(mut self, updated_at: NaiveDateTime) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    pub fn max_features(mut self, max_features: i16) -> Self {
        self.krate.max_features = Some(max_features);
        self
    }

    pub async fn build(mut self, connection: &mut AsyncPgConnection) -> AppResult<Crate> {
        use diesel::{insert_into, select, update};
        use diesel_async::RunQueryDsl;

        let mut krate = self.krate.create(connection, self.owner_id).await?;

        // Since we are using `NewCrate`, we can't set all the
        // crate properties in a single DB call.

        if let Some(downloads) = self.downloads {
            update(crate_downloads::table.filter(crate_downloads::crate_id.eq(krate.id)))
                .set(crate_downloads::downloads.eq(downloads as i64))
                .execute(connection)
                .await?;
        }

        if self.versions.is_empty() {
            self.versions.push(VersionBuilder::new("0.99.0"));
        }

        let mut last_version_id = 0;
        for version_builder in self.versions {
            last_version_id = version_builder
                .build(krate.id, self.owner_id, connection)
                .await?
                .id;
        }

        if let Some(downloads) = self.recent_downloads {
            insert_into(version_downloads::table)
                .values((
                    version_downloads::version_id.eq(last_version_id),
                    version_downloads::downloads.eq(downloads),
                ))
                .execute(connection)
                .await?;

            define_sql_function!(fn refresh_recent_crate_downloads());
            select(refresh_recent_crate_downloads())
                .execute(connection)
                .await?;
        }

        if !self.categories.is_empty() {
            Category::update_crate(connection, krate.id, &self.categories).await?;
        }

        if !self.keywords.is_empty() {
            Keyword::update_crate(connection, krate.id, &self.keywords).await?;
        }

        if let Some(updated_at) = self.updated_at {
            krate = update(&krate)
                .set(crates::updated_at.eq(updated_at))
                .returning(Crate::as_returning())
                .get_result(connection)
                .await?;
        }

        update_default_version(krate.id, connection).await?;

        Ok(krate)
    }

    /// Consumes the builder and creates the crate record in the database.
    ///
    /// # Panics
    ///
    /// Panics (and fails the test) if any part of inserting the crate record fails.
    pub async fn expect_build(self, connection: &mut AsyncPgConnection) -> Crate {
        let name = self.krate.name;
        self.build(connection).await.unwrap_or_else(|e| {
            panic!("Unable to create crate {name}: {e:?}");
        })
    }
}
