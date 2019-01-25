//! Structs using the builder pattern that make it easier to create records in tests.

use std::collections::HashMap;
use std::io::Read;

use chrono;
use chrono::Utc;
use diesel::prelude::*;
use flate2::write::GzEncoder;
use flate2::Compression;
use semver;
use tar;

use cargo_registry::util::CargoResult;

use models::{Crate, CrateDownload, Keyword, Version};
use models::{NewCrate, NewVersion};
use schema::*;
use views::krate_publish as u;

/// A builder to create version records for the purpose of inserting directly into the database.
pub struct VersionBuilder<'a> {
    num: semver::Version,
    license: Option<&'a str>,
    license_file: Option<&'a str>,
    features: HashMap<String, Vec<String>>,
    dependencies: Vec<(i32, Option<&'static str>)>,
    yanked: bool,
}

impl<'a> VersionBuilder<'a> {
    /// Creates a VersionBuilder from a string slice `num` representing the version's number.
    ///
    /// # Panics
    ///
    /// Panics if `num` cannot be parsed as a valid `semver::Version`.
    pub fn new(num: &str) -> Self {
        let num = semver::Version::parse(num).unwrap_or_else(|e| {
            panic!("The version {} is not valid: {}", num, e);
        });

        VersionBuilder {
            num,
            license: None,
            license_file: None,
            features: HashMap::new(),
            dependencies: Vec::new(),
            yanked: false,
        }
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

    fn build(self, crate_id: i32, connection: &PgConnection) -> CargoResult<Version> {
        use diesel::{insert_into, update};

        let license = match self.license {
            Some(license) => Some(license.to_owned()),
            None => None,
        };

        let mut vers = NewVersion::new(
            crate_id,
            &self.num,
            &self.features,
            license,
            self.license_file,
            None,
        )?
        .save(connection, &[])?;

        if self.yanked {
            vers = update(&vers)
                .set(versions::yanked.eq(true))
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
}

impl<'a> From<&'a str> for VersionBuilder<'a> {
    fn from(num: &'a str) -> Self {
        VersionBuilder::new(num)
    }
}

/// A builder to create crate records for the purpose of inserting directly into the database.
/// If you want to test logic that happens as part of a publish request, use `PublishBuilder`
/// instead.
pub struct CrateBuilder<'a> {
    owner_id: i32,
    krate: NewCrate<'a>,
    downloads: Option<i32>,
    recent_downloads: Option<i32>,
    versions: Vec<VersionBuilder<'a>>,
    keywords: Vec<&'a str>,
}

impl<'a> CrateBuilder<'a> {
    /// Create a new instance with the given crate name and owner. If the owner with the given ID
    /// doesn't exist in the database, `expect_build` will fail.
    pub fn new(name: &str, owner_id: i32) -> CrateBuilder<'_> {
        CrateBuilder {
            owner_id,
            krate: NewCrate {
                name,
                ..NewCrate::default()
            },
            downloads: None,
            recent_downloads: None,
            versions: Vec::new(),
            keywords: Vec::new(),
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
    pub fn version<T: Into<VersionBuilder<'a>>>(mut self, version: T) -> Self {
        self.versions.push(version.into());
        self
    }

    /// Adds a keyword to the crate.
    pub fn keyword(mut self, keyword: &'a str) -> Self {
        self.keywords.push(keyword);
        self
    }

    fn build(mut self, connection: &PgConnection) -> CargoResult<Crate> {
        use diesel::{insert_into, select, update};

        let mut krate = self
            .krate
            .create_or_update(connection, None, self.owner_id)?;

        // Since we are using `NewCrate`, we can't set all the
        // crate properties in a single DB call.

        let old_downloads = self.downloads.unwrap_or(0) - self.recent_downloads.unwrap_or(0);
        let now = Utc::now();
        let old_date = now.naive_utc().date() - chrono::Duration::days(91);

        if let Some(downloads) = self.downloads {
            let crate_download = CrateDownload {
                crate_id: krate.id,
                downloads: old_downloads,
                date: old_date,
            };

            insert_into(crate_downloads::table)
                .values(&crate_download)
                .execute(connection)?;
            krate.downloads = downloads;
            update(&krate).set(&krate).execute(connection)?;
        }

        if self.recent_downloads.is_some() {
            let crate_download = CrateDownload {
                crate_id: krate.id,
                downloads: self.recent_downloads.unwrap(),
                date: now.naive_utc().date(),
            };

            insert_into(crate_downloads::table)
                .values(&crate_download)
                .execute(connection)?;

            no_arg_sql_function!(refresh_recent_crate_downloads, ());
            select(refresh_recent_crate_downloads).execute(connection)?;
        }

        if self.versions.is_empty() {
            self.versions.push(VersionBuilder::new("0.99.0"));
        }

        for version_builder in self.versions {
            version_builder.build(krate.id, connection)?;
        }

        if !self.keywords.is_empty() {
            Keyword::update_crate(connection, &krate, &self.keywords)?;
        }

        Ok(krate)
    }

    /// Consumes the builder and creates the crate record in the database.
    ///
    /// # Panics
    ///
    /// Panics (and fails the test) if any part of inserting the crate record fails.
    pub fn expect_build(self, connection: &PgConnection) -> Crate {
        let name = self.krate.name;
        self.build(connection).unwrap_or_else(|e| {
            panic!("Unable to create crate {}: {:?}", name, e);
        })
    }
}

lazy_static! {
    // The bytes of an empty tarball is not an empty vector of bytes because of tarball headers.
    // Unless files are added to a PublishBuilder, the `.crate` tarball that gets uploaded
    // will be empty, so precompute the empty tarball bytes to use as a default.
    static ref EMPTY_TARBALL_BYTES: Vec<u8> = {
        let mut empty_tarball = vec![];
        {
            let mut ar =
                tar::Builder::new(GzEncoder::new(&mut empty_tarball, Compression::default()));
            t!(ar.finish());
        }
        empty_tarball
    };
}

/// A builder for constructing a crate for the purposes of testing publishing. If you only need
/// a crate to exist and don't need to test behavior caused by the publish request, inserting
/// a crate into the database directly by using CrateBuilder will be faster.
pub struct PublishBuilder {
    pub krate_name: String,
    version: semver::Version,
    tarball: Vec<u8>,
    deps: Vec<u::CrateDependency>,
    desc: Option<String>,
}

impl PublishBuilder {
    /// Create a request to publish a crate with the given name, version 1.0.0, and no files
    /// in its tarball.
    pub fn new(krate_name: &str) -> Self {
        PublishBuilder {
            krate_name: krate_name.into(),
            version: semver::Version::parse("1.0.0").unwrap(),
            tarball: EMPTY_TARBALL_BYTES.to_vec(),
            deps: vec![],
            desc: Some("description".to_string()),
        }
    }

    /// Set the version of the crate being published to something other than the default of 1.0.0.
    pub fn version(mut self, version: &str) -> Self {
        self.version = semver::Version::parse(version).unwrap();
        self
    }

    /// Set the files in the crate's tarball.
    pub fn files(mut self, files: &[(&str, &[u8])]) -> Self {
        let mut slices = files.iter().map(|p| p.1).collect::<Vec<_>>();
        let files = files
            .iter()
            .zip(&mut slices)
            .map(|(&(name, _), data)| {
                let len = data.len() as u64;
                (name, data as &mut Read, len)
            })
            .collect::<Vec<_>>();

        let mut tarball = Vec::new();
        {
            let mut ar = tar::Builder::new(GzEncoder::new(&mut tarball, Compression::default()));
            for (name, ref mut data, size) in files {
                let mut header = tar::Header::new_gnu();
                t!(header.set_path(name));
                header.set_size(size);
                header.set_cksum();
                t!(ar.append(&header, data));
            }
            t!(ar.finish());
        }

        self.tarball = tarball;
        self
    }

    /// Add a dependency to this crate. Make sure the dependency already exists in the
    /// database or publish will fail.
    pub fn dependency(mut self, dep: DependencyBuilder) -> Self {
        self.deps.push(dep.build());
        self
    }

    /// Set the description of this crate
    pub fn description(mut self, description: &str) -> Self {
        self.desc = Some(description.to_string());
        self
    }

    /// Consume this builder to make the Put request body
    pub fn body(self) -> Vec<u8> {
        let new_crate = u::NewCrate {
            name: u::CrateName(self.krate_name.clone()),
            vers: u::CrateVersion(self.version),
            features: HashMap::new(),
            deps: self.deps,
            authors: vec!["foo".to_string()],
            description: self.desc,
            homepage: None,
            documentation: None,
            readme: None,
            readme_file: None,
            keywords: Some(u::KeywordList(Vec::new())),
            categories: Some(u::CategoryList(Vec::new())),
            license: Some("MIT".to_string()),
            license_file: None,
            repository: None,
            badges: Some(HashMap::new()),
            links: None,
        };

        ::new_crate_to_body_with_tarball(&new_crate, &self.tarball)
    }
}

/// A builder for constructing a dependency of another crate.
pub struct DependencyBuilder {
    name: String,
    explicit_name_in_toml: Option<u::CrateName>,
    version_req: u::CrateVersionReq,
}

impl DependencyBuilder {
    /// Create a dependency on the crate with the given name.
    pub fn new(name: &str) -> Self {
        DependencyBuilder {
            name: name.to_string(),
            explicit_name_in_toml: None,
            version_req: u::CrateVersionReq(semver::VersionReq::parse(">= 0").unwrap()),
        }
    }

    /// Rename this dependency.
    pub fn rename(mut self, new_name: &str) -> Self {
        self.explicit_name_in_toml = Some(u::CrateName(new_name.to_string()));
        self
    }

    /// Set the version requirement for this dependency.
    ///
    /// # Panics
    ///
    /// Panics if the `version_req` string specified isn't a valid `semver::VersionReq`.
    pub fn version_req(mut self, version_req: &str) -> Self {
        self.version_req = u::CrateVersionReq(
            semver::VersionReq::parse(version_req)
                .expect("version req isn't a valid semver::VersionReq"),
        );
        self
    }

    /// Consume this builder to create a `u::CrateDependency`. If the dependent crate doesn't
    /// already exist, publishing a crate with this dependency will fail.
    fn build(self) -> u::CrateDependency {
        u::CrateDependency {
            name: u::CrateName(self.name),
            optional: false,
            default_features: true,
            features: Vec::new(),
            version_req: self.version_req,
            target: None,
            kind: None,
            explicit_name_in_toml: self.explicit_name_in_toml,
        }
    }
}
