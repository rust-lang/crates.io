use crates_io::views::krate_publish as u;
use std::collections::BTreeMap;

use crates_io_tarball::TarballBuilder;
use flate2::{write::GzEncoder, Compression};

use super::DependencyBuilder;

/// A builder for constructing a crate for the purposes of testing publishing. If you only need
/// a crate to exist and don't need to test behavior caused by the publish request, inserting
/// a crate into the database directly by using CrateBuilder will be faster.
pub struct PublishBuilder {
    categories: Vec<String>,
    deps: Vec<u::EncodableCrateDependency>,
    desc: Option<String>,
    doc_url: Option<String>,
    keywords: Vec<String>,
    pub krate_name: String,
    license: Option<String>,
    license_file: Option<String>,
    readme: Option<String>,
    tarball: Vec<u8>,
    version: semver::Version,
    features: BTreeMap<u::EncodableFeatureName, Vec<u::EncodableFeature>>,
}

impl PublishBuilder {
    /// Create a request to publish a crate with the given name and version, and no files
    /// in its tarball.
    pub fn new(krate_name: &str, version: &str) -> Self {
        let tarball = TarballBuilder::new(krate_name, version)
            .add_raw_manifest(b"[package]")
            .build();

        PublishBuilder {
            categories: vec![],
            deps: vec![],
            desc: Some("description".to_string()),
            doc_url: None,
            keywords: vec![],
            krate_name: krate_name.into(),
            license: Some("MIT".to_string()),
            license_file: None,
            readme: None,
            tarball,
            version: semver::Version::parse(version).unwrap(),
            features: BTreeMap::new(),
        }
    }

    /// Set the files in the crate's tarball.
    pub fn files(mut self, files: &[(&str, &[u8])]) -> Self {
        let mut tarball = Vec::new();
        {
            let mut ar = tar::Builder::new(GzEncoder::new(&mut tarball, Compression::default()));
            for (name, data) in files {
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                assert_ok!(ar.append_data(&mut header, name, *data));
            }
            assert_ok!(ar.finish());
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

    // Adds a feature.
    pub fn feature(mut self, name: &str, values: &[&str]) -> Self {
        let values = values
            .iter()
            .map(|s| u::EncodableFeature(s.to_string()))
            .collect();
        self.features
            .insert(u::EncodableFeatureName(name.to_string()), values);
        self
    }

    pub fn build(self) -> (String, Vec<u8>) {
        let new_crate = u::EncodableCrateUpload {
            name: u::EncodableCrateName(self.krate_name.clone()),
            vers: u::EncodableCrateVersion(self.version),
            features: self.features,
            deps: self.deps,
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
            links: None,
        };

        (serde_json::to_string(&new_crate).unwrap(), self.tarball)
    }

    /// Consume this builder to make the Put request body
    pub fn body(self) -> Vec<u8> {
        let (json, tarball) = self.build();
        PublishBuilder::create_publish_body(&json, &tarball)
    }

    pub fn create_publish_body(json: &str, tarball: &[u8]) -> Vec<u8> {
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

        body.extend([
            tarball.len() as u8,
            (tarball.len() >> 8) as u8,
            (tarball.len() >> 16) as u8,
            (tarball.len() >> 24) as u8,
        ]);
        body.extend(tarball);
        body
    }
}
