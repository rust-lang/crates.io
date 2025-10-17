use crates_io_api_types::krate_publish;

/// A builder for constructing a dependency of another crate.
pub struct DependencyBuilder {
    explicit_name_in_toml: Option<String>,
    name: String,
    features: Vec<String>,
    registry: Option<String>,
    version_req: String,
}

impl DependencyBuilder {
    /// Create a dependency on the crate with the given name.
    pub fn new(name: &str) -> Self {
        DependencyBuilder {
            explicit_name_in_toml: None,
            name: name.to_string(),
            features: vec![],
            registry: None,
            version_req: "> 0".to_string(),
        }
    }

    /// Rename this dependency.
    pub fn rename(mut self, new_name: &str) -> Self {
        self.explicit_name_in_toml = Some(new_name.to_string());
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
    #[track_caller]
    pub fn version_req(mut self, version_req: &str) -> Self {
        self.version_req = version_req.to_string();
        self
    }

    pub fn add_feature<T: Into<String>>(mut self, feature: T) -> Self {
        self.features.push(feature.into());
        self
    }

    /// Consume this builder to create a `krate_publish::EncodableCrateDependency`. If the dependent crate doesn't
    /// already exist, publishing a crate with this dependency will fail.
    pub fn build(self) -> krate_publish::EncodableCrateDependency {
        krate_publish::EncodableCrateDependency {
            name: self.name,
            optional: false,
            default_features: true,
            features: self.features,
            version_req: self.version_req,
            target: None,
            kind: None,
            explicit_name_in_toml: self.explicit_name_in_toml,
            registry: self.registry,
        }
    }
}
