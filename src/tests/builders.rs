//! Structs using the builder pattern that make it easier to create records in tests.

use cargo_registry::{
    models::{Crate, Keyword, NewCrate, NewVersion, Version},
    schema::{crates, dependencies, version_downloads, versions},
    util::CargoResult,
    views::krate_publish as u,
};
use std::{collections::HashMap, io::Read};

use diesel::prelude::*;
use flate2::{write::GzEncoder, Compression};

/// A builder to create version records for the purpose of inserting directly into the database.
pub struct VersionBuilder<'a> {
    num: semver::Version,
    license: Option<&'a str>,
    license_file: Option<&'a str>,
    features: HashMap<String, Vec<String>>,
    dependencies: Vec<(i32, Option<&'static str>)>,
    yanked: bool,
    size: i32,
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
            size: 0,
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

    /// Sets the version's size.
    pub fn size(mut self, size: i32) -> Self {
        self.size = size;
        self
    }

    fn build(
        self,
        crate_id: i32,
        published_by: i32,
        connection: &PgConnection,
    ) -> CargoResult<Version> {
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
            self.size,
            published_by,
        )?
        .save(connection, &[], "someone@example.com")?;

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

    /// Consumes the builder and creates the version record in the database.
    ///
    /// # Panics
    ///
    /// Panics (and fails the test) if any part of inserting the version record fails.
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
            .create_or_update(connection, None, self.owner_id, None)?;

        // Since we are using `NewCrate`, we can't set all the
        // crate properties in a single DB call.

        if let Some(downloads) = self.downloads {
            krate = update(&krate)
                .set(crates::downloads.eq(downloads))
                .returning(cargo_registry::models::krate::ALL_COLUMNS)
                .get_result(connection)?;
        }

        if self.versions.is_empty() {
            self.versions.push(VersionBuilder::new("0.99.0"));
        }

        let mut last_version_id = 0;
        for version_builder in self.versions {
            last_version_id = version_builder
                .build(krate.id, self.owner_id, connection)?
                .id;
        }

        if let Some(downloads) = self.recent_downloads {
            insert_into(version_downloads::table)
                .values((
                    version_downloads::version_id.eq(last_version_id),
                    version_downloads::downloads.eq(downloads),
                ))
                .execute(connection)?;

            no_arg_sql_function!(refresh_recent_crate_downloads, ());
            select(refresh_recent_crate_downloads).execute(connection)?;
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
    deps: Vec<u::EncodableCrateDependency>,
    desc: Option<String>,
    readme: Option<String>,
    doc_url: Option<String>,
    keywords: Vec<String>,
    categories: Vec<String>,
    badges: HashMap<String, HashMap<String, String>>,
    license: Option<String>,
    license_file: Option<String>,
    authors: Vec<String>,
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
            readme: None,
            doc_url: None,
            keywords: vec![],
            categories: vec![],
            badges: HashMap::new(),
            license: Some("MIT".to_string()),
            license_file: None,
            authors: vec!["foo".to_string()],
        }
    }

    /// Set the version of the crate being published to something other than the default of 1.0.0.
    pub fn version(mut self, version: &str) -> Self {
        self.version = semver::Version::parse(version).unwrap();
        self
    }

    /// Set the files in the crate's tarball.
    pub fn files(self, files: &[(&str, &[u8])]) -> Self {
        let mut slices = files.iter().map(|p| p.1).collect::<Vec<_>>();
        let mut files = files
            .iter()
            .zip(&mut slices)
            .map(|(&(name, _), data)| {
                let len = data.len() as u64;
                (name, data as &mut dyn Read, len)
            })
            .collect::<Vec<_>>();

        self.files_with_io(&mut files)
    }

    /// Set the tarball from a Read trait object
    pub fn files_with_io(mut self, files: &mut [(&str, &mut dyn Read, u64)]) -> Self {
        let mut tarball = Vec::new();
        {
            let mut ar = tar::Builder::new(GzEncoder::new(&mut tarball, Compression::default()));
            for &mut (name, ref mut data, size) in files {
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

    /// Set the tarball directly to the given Vec of bytes
    pub fn tarball(mut self, tarball: Vec<u8>) -> Self {
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

    /// Unset the description of this crate. Publish will fail unless description is reset.
    pub fn unset_description(mut self) -> Self {
        self.desc = None;
        self
    }

    /// Set the readme of this crate
    pub fn readme(mut self, readme: &str) -> Self {
        self.readme = Some(readme.to_string());
        self
    }

    /// Set the documentation URL of this crate
    pub fn documentation(mut self, documentation: &str) -> Self {
        self.doc_url = Some(documentation.to_string());
        self
    }

    /// Add a keyword to this crate.
    pub fn keyword(mut self, keyword: &str) -> Self {
        self.keywords.push(keyword.into());
        self
    }

    /// Add a category to this crate. Make sure the category already exists in the
    /// database or it will be ignored.
    pub fn category(mut self, slug: &str) -> Self {
        self.categories.push(slug.into());
        self
    }

    /// Add badges to this crate.
    pub fn badges(mut self, badges: HashMap<String, HashMap<String, String>>) -> Self {
        self.badges = badges;
        self
    }

    /// Remove the license from this crate. Publish will fail unless license or license file is set.
    pub fn unset_license(mut self) -> Self {
        self.license = None;
        self
    }

    /// Set the license file for this crate
    pub fn license_file(mut self, license_file: &str) -> Self {
        self.license_file = Some(license_file.into());
        self
    }

    /// Add an author to this crate
    pub fn author(mut self, author: &str) -> Self {
        self.authors.push(author.into());
        self
    }

    /// Remove the authors from this crate. Publish will fail unless authors are reset.
    pub fn unset_authors(mut self) -> Self {
        self.authors = vec![];
        self
    }

    /// Consume this builder to make the Put request body
    pub fn body(self) -> Vec<u8> {
        let new_crate = u::EncodableCrateUpload {
            name: u::EncodableCrateName(self.krate_name.clone()),
            vers: u::EncodableCrateVersion(self.version),
            features: HashMap::new(),
            deps: self.deps,
            authors: self.authors,
            description: self.desc,
            homepage: None,
            documentation: self.doc_url,
            readme: self.readme,
            readme_file: None,
            keywords: u::EncodableKeywordList(
                self.keywords.into_iter().map(u::EncodableKeyword).collect(),
            ),
            categories: u::EncodableCategoryList(
                self.categories
                    .into_iter()
                    .map(u::EncodableCategory)
                    .collect(),
            ),
            license: self.license,
            license_file: self.license_file,
            repository: None,
            badges: Some(self.badges),
            links: None,
        };

        let json = serde_json::to_string(&new_crate).unwrap();
        let mut body = Vec::new();
        body.extend(
            [
                json.len() as u8,
                (json.len() >> 8) as u8,
                (json.len() >> 16) as u8,
                (json.len() >> 24) as u8,
            ]
            .iter()
            .cloned(),
        );
        body.extend(json.as_bytes().iter().cloned());

        let tarball = &self.tarball;
        body.extend(&[
            tarball.len() as u8,
            (tarball.len() >> 8) as u8,
            (tarball.len() >> 16) as u8,
            (tarball.len() >> 24) as u8,
        ]);
        body.extend(tarball);
        body
    }
}

/// A builder for constructing a dependency of another crate.
pub struct DependencyBuilder {
    name: String,
    registry: Option<String>,
    explicit_name_in_toml: Option<u::EncodableCrateName>,
    version_req: u::EncodableCrateVersionReq,
}

impl DependencyBuilder {
    /// Create a dependency on the crate with the given name.
    pub fn new(name: &str) -> Self {
        DependencyBuilder {
            name: name.to_string(),
            registry: None,
            explicit_name_in_toml: None,
            version_req: u::EncodableCrateVersionReq(semver::VersionReq::parse(">= 0").unwrap()),
        }
    }

    /// Rename this dependency.
    pub fn rename(mut self, new_name: &str) -> Self {
        self.explicit_name_in_toml = Some(u::EncodableCrateName(new_name.to_string()));
        self
    }

    /// Set an alternative registry for this dependency.
    pub fn registry(mut self, registry: &str) -> Self {
        self.registry = Some(registry.to_string());
        self
    }

    /// Set the version requirement for this dependency.
    ///
    /// # Panics
    ///
    /// Panics if the `version_req` string specified isn't a valid `semver::VersionReq`.
    pub fn version_req(mut self, version_req: &str) -> Self {
        self.version_req = u::EncodableCrateVersionReq(
            semver::VersionReq::parse(version_req)
                .expect("version req isn't a valid semver::VersionReq"),
        );
        self
    }

    /// Consume this builder to create a `u::CrateDependency`. If the dependent crate doesn't
    /// already exist, publishing a crate with this dependency will fail.
    fn build(self) -> u::EncodableCrateDependency {
        u::EncodableCrateDependency {
            name: u::EncodableCrateName(self.name),
            optional: false,
            default_features: true,
            features: Vec::new(),
            version_req: self.version_req,
            target: None,
            kind: None,
            explicit_name_in_toml: self.explicit_name_in_toml,
            registry: self.registry,
        }
    }
}
