//! This module handles the expected information a crate should have
//! and manages the serialising and deserializing of this information
//! to and from structs. The serializing is only utilised in
//! integration tests.

use serde::{Deserialize, Serialize};

use crate::models::DependencyKind;

#[derive(Deserialize, Serialize, Debug)]
pub struct PublishMetadata {
    pub name: String,
    pub vers: String,
    pub readme: Option<String>,
    pub readme_file: Option<String>,
}

#[derive(Debug)]
pub struct EncodableCrateDependency {
    pub optional: bool,
    pub default_features: bool,
    pub name: String,
    pub features: Vec<String>,
    pub version_req: String,
    pub target: Option<String>,
    pub kind: Option<DependencyKind>,
    pub explicit_name_in_toml: Option<String>,
    pub registry: Option<String>,
}
