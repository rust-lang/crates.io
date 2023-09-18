use cargo_manifest::{DependencyDetail, DepsSet, FeatureSet, MaybeInherited};
use crates_io::models::DependencyKind;
use crates_io::views::krate_publish as u;
use hyper::body::Bytes;
use std::collections::BTreeMap;

use crates_io_tarball::TarballBuilder;

use super::DependencyBuilder;

/// A builder for constructing a crate for the purposes of testing publishing. If you only need
/// a crate to exist and don't need to test behavior caused by the publish request, inserting
/// a crate into the database directly by using CrateBuilder will be faster.
pub struct PublishBuilder {
    categories: Vec<String>,
    deps: Vec<u::EncodableCrateDependency>,
    desc: Option<String>,
    doc_url: Option<String>,
    files: Vec<(String, Bytes)>,
    keywords: Vec<String>,
    krate_name: String,
    license: Option<String>,
    license_file: Option<String>,
    manifest: Manifest,
    readme: Option<String>,
    version: semver::Version,
    features: BTreeMap<u::EncodableFeatureName, Vec<u::EncodableFeature>>,
}

enum Manifest {
    None,
    Generated,
    Custom(Bytes),
}

impl PublishBuilder {
    /// Create a request to publish a crate with the given name and version, and no files
    /// in its tarball.
    pub fn new(krate_name: &str, version: &str) -> Self {
        PublishBuilder {
            categories: vec![],
            deps: vec![],
            desc: Some("description".to_string()),
            doc_url: None,
            files: vec![],
            keywords: vec![],
            krate_name: krate_name.into(),
            license: Some("MIT".to_string()),
            license_file: None,
            manifest: Manifest::Generated,
            readme: None,
            version: semver::Version::parse(version).unwrap(),
            features: BTreeMap::new(),
        }
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

    pub fn no_manifest(mut self) -> Self {
        self.manifest = Manifest::None;
        self
    }

    pub fn custom_manifest(mut self, manifest: impl Into<Bytes>) -> Self {
        self.manifest = Manifest::Custom(manifest.into());
        self
    }

    pub fn add_file(mut self, path: impl ToString, content: impl Into<Bytes>) -> Self {
        self.files.push((path.to_string(), content.into()));
        self
    }

    pub fn build(self) -> (String, Vec<u8>) {
        let new_crate = u::EncodableCrateUpload {
            name: u::EncodableCrateName(self.krate_name.clone()),
            vers: u::EncodableCrateVersion(self.version.clone()),
            features: self.features.clone(),
            deps: self.deps.clone(),
            description: self.desc.clone(),
            homepage: None,
            documentation: self.doc_url.clone(),
            readme: self.readme,
            readme_file: None,
            keywords: u::EncodableKeywordList(
                self.keywords
                    .clone()
                    .into_iter()
                    .map(u::EncodableKeyword)
                    .collect(),
            ),
            categories: u::EncodableCategoryList(
                self.categories
                    .clone()
                    .into_iter()
                    .map(u::EncodableCategory)
                    .collect(),
            ),
            license: self.license.clone(),
            license_file: self.license_file.clone(),
            repository: None,
            links: None,
        };

        let mut tarball_builder = TarballBuilder::new(&self.krate_name, &self.version.to_string());

        match self.manifest {
            Manifest::None => {}
            Manifest::Generated => {
                let mut package = cargo_manifest::Package::<()>::new(
                    self.krate_name.clone(),
                    self.version.to_string(),
                );
                package.categories = self.categories.none_or_filled().map(MaybeInherited::Local);
                package.description = self.desc.map(MaybeInherited::Local);
                package.documentation = self.doc_url.map(MaybeInherited::Local);
                package.keywords = self.keywords.none_or_filled().map(MaybeInherited::Local);
                package.license = self.license.map(MaybeInherited::Local);
                package.license_file = self.license_file.map(MaybeInherited::Local);

                let mut build_deps = DepsSet::new();
                let mut deps = DepsSet::new();
                let mut dev_deps = DepsSet::new();

                for encoded in self.deps {
                    let (name, dependency) = convert_dependency(&encoded);
                    match encoded.kind {
                        Some(DependencyKind::Build) => build_deps.insert(name, dependency),
                        None | Some(DependencyKind::Normal) => deps.insert(name, dependency),
                        Some(DependencyKind::Dev) => dev_deps.insert(name, dependency),
                    };
                }

                let manifest = cargo_manifest::Manifest {
                    package: Some(package),
                    build_dependencies: build_deps.none_or_filled(),
                    dependencies: deps.none_or_filled(),
                    dev_dependencies: dev_deps.none_or_filled(),
                    features: convert_features(self.features).none_or_filled(),
                    ..Default::default()
                };

                let manifest = toml::to_string(&manifest).unwrap();

                tarball_builder = tarball_builder.add_raw_manifest(manifest.as_bytes());
            }
            Manifest::Custom(bytes) => {
                tarball_builder = tarball_builder.add_raw_manifest(&bytes);
            }
        }

        for (path, content) in self.files {
            tarball_builder = tarball_builder.add_file(&path, &content);
        }

        let tarball = tarball_builder.build();
        (serde_json::to_string(&new_crate).unwrap(), tarball)
    }

    /// Consume this builder to make the Put request body
    pub fn body(self) -> Vec<u8> {
        let (json, tarball) = self.build();
        PublishBuilder::create_publish_body(&json, &tarball)
    }

    pub fn create_publish_body(json: &str, tarball: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();

        let json_len = json.len();
        body.push(json_len as u8);
        body.push((json_len >> 8) as u8);
        body.push((json_len >> 16) as u8);
        body.push((json_len >> 24) as u8);

        body.extend(json.as_bytes());

        let tarball_len = tarball.len();
        body.push(tarball_len as u8);
        body.push((tarball_len >> 8) as u8);
        body.push((tarball_len >> 16) as u8);
        body.push((tarball_len >> 24) as u8);

        body.extend(tarball);

        body
    }
}

fn convert_dependency(
    encoded: &u::EncodableCrateDependency,
) -> (String, cargo_manifest::Dependency) {
    if is_simple_dependency(encoded) {
        let dependency = cargo_manifest::Dependency::Simple(encoded.version_req.to_string());
        return (encoded.name.to_string(), dependency);
    }

    let (name, package) = match encoded.explicit_name_in_toml.as_ref() {
        None => (encoded.name.to_string(), None),
        Some(explicit_name_in_toml) => (
            explicit_name_in_toml.to_string(),
            Some(encoded.name.to_string()),
        ),
    };

    let features = encoded
        .features
        .iter()
        .map(|f| f.to_string())
        .collect::<Vec<_>>()
        .none_or_filled();

    let dependency = DependencyDetail {
        version: Some(encoded.version_req.to_string()),
        registry: encoded.registry.clone(),
        features,
        optional: match encoded.optional {
            true => Some(true),
            false => None,
        },
        default_features: match encoded.default_features {
            true => None,
            false => Some(false),
        },
        package,
        ..Default::default()
    };

    let dependency = cargo_manifest::Dependency::Detailed(dependency);
    (name, dependency)
}

fn is_simple_dependency(dep: &u::EncodableCrateDependency) -> bool {
    !dep.optional
        && dep.default_features
        && dep.features.is_empty()
        && dep.target.is_none()
        && dep.explicit_name_in_toml.is_none()
        && dep.registry.is_none()
}

fn convert_features(
    encoded: BTreeMap<u::EncodableFeatureName, Vec<u::EncodableFeature>>,
) -> FeatureSet {
    encoded
        .into_iter()
        .map(|(key, value)| (key.0, value.into_iter().map(|f| f.0).collect()))
        .collect()
}

trait NoneOrFilled: Sized {
    fn none_or_filled(self) -> Option<Self>;
}

impl<T> NoneOrFilled for Vec<T> {
    fn none_or_filled(self) -> Option<Self> {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

impl<K, V> NoneOrFilled for BTreeMap<K, V> {
    fn none_or_filled(self) -> Option<Self> {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}
