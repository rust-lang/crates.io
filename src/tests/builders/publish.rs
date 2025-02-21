use crate::models::DependencyKind;
use crate::views::krate_publish as u;
use bytes::{BufMut, Bytes, BytesMut};
use cargo_manifest::{DependencyDetail, DepsSet, MaybeInherited};
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
    features: BTreeMap<String, Vec<String>>,
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

    /// Set the license from this crate.
    pub fn license<T: Into<String>>(mut self, license: T) -> Self {
        self.license = Some(license.into());
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
        let values = values.iter().map(ToString::to_string).collect();
        self.features.insert(name.to_string(), values);
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
        let metadata = u::PublishMetadata {
            name: self.krate_name.clone(),
            vers: self.version.to_string(),
            readme: self.readme,
            readme_file: None,
        };

        let mut tarball_builder = TarballBuilder::new();

        match self.manifest {
            Manifest::None => {}
            Manifest::Generated => {
                let mut package =
                    cargo_manifest::Package::new(self.krate_name.clone(), self.version.to_string());
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

                let manifest = cargo_manifest::Manifest::<(), ()> {
                    package: Some(package),
                    build_dependencies: build_deps.none_or_filled(),
                    dependencies: deps.none_or_filled(),
                    dev_dependencies: dev_deps.none_or_filled(),
                    features: self.features.none_or_filled(),
                    ..Default::default()
                };

                let manifest = toml::to_string(&manifest).unwrap();

                let content = manifest.as_bytes();

                let path = format!("{}-{}/Cargo.toml", self.krate_name, self.version);
                tarball_builder = tarball_builder.add_file(&path, content);
            }
            Manifest::Custom(bytes) => {
                let path = format!("{}-{}/Cargo.toml", self.krate_name, self.version);
                tarball_builder = tarball_builder.add_file(&path, &bytes);
            }
        }

        for (path, content) in self.files {
            tarball_builder = tarball_builder.add_file(&path, &content);
        }

        let tarball = tarball_builder.build();
        (serde_json::to_string(&metadata).unwrap(), tarball)
    }

    /// Consume this builder to make the Put request body
    pub fn body(self) -> Bytes {
        let (json, tarball) = self.build();
        PublishBuilder::create_publish_body(&json, &tarball)
    }

    pub fn create_publish_body(json: &str, tarball: &[u8]) -> Bytes {
        let json_len = json.len();
        let tarball_len = tarball.len();

        let mut body = BytesMut::with_capacity(json_len + tarball_len + 2);
        body.put_u32_le(json_len as u32);
        body.put_slice(json.as_bytes());
        body.put_u32_le(tarball_len as u32);
        body.put_slice(tarball);

        body.freeze()
    }
}

impl From<PublishBuilder> for Bytes {
    fn from(builder: PublishBuilder) -> Self {
        builder.body()
    }
}

fn convert_dependency(
    encoded: &u::EncodableCrateDependency,
) -> (String, cargo_manifest::Dependency) {
    let (name, package) = match encoded.explicit_name_in_toml.as_ref() {
        None => (encoded.name.to_string(), None),
        Some(explicit_name_in_toml) => (
            explicit_name_in_toml.to_string(),
            Some(encoded.name.to_string()),
        ),
    };

    let dependency = DependencyDetail {
        version: Some(encoded.version_req.to_string()),
        registry: encoded.registry.clone(),
        features: encoded.features.clone().none_or_filled(),
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

    let dependency = cargo_manifest::Dependency::Detailed(dependency).simplify();
    (name, dependency)
}

trait NoneOrFilled: Sized {
    fn none_or_filled(self) -> Option<Self>;
}

impl<T> NoneOrFilled for Vec<T> {
    fn none_or_filled(self) -> Option<Self> {
        if self.is_empty() { None } else { Some(self) }
    }
}

impl<K, V> NoneOrFilled for BTreeMap<K, V> {
    fn none_or_filled(self) -> Option<Self> {
        if self.is_empty() { None } else { Some(self) }
    }
}
