//! This module handles the expected information a crate should have
//! and manages the serialising and deserializing of this information
//! to and from structs. The serializing is only utilised in
//! integration tests.

use diesel::pg::Pg;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use serde::{de, Deserialize, Deserializer, Serialize};

use crate::models::krate::MAX_NAME_LENGTH;

use crate::models::Crate;
use crate::models::DependencyKind;

#[derive(Deserialize, Serialize, Debug)]
pub struct PublishMetadata {
    pub name: EncodableCrateName,
    pub vers: EncodableCrateVersion,
    pub deps: Vec<EncodableCrateDependency>,
    pub readme: Option<String>,
    pub readme_file: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EncodableCrateDependency {
    pub optional: bool,
    pub default_features: bool,
    pub name: EncodableCrateName,
    pub features: Vec<EncodableFeature>,
    pub version_req: EncodableCrateVersionReq,
    pub target: Option<String>,
    pub kind: Option<DependencyKind>,
    pub explicit_name_in_toml: Option<EncodableDependencyName>,
    pub registry: Option<String>,
}

#[derive(PartialEq, Eq, Hash, Serialize, Clone, Debug, Deref)]
pub struct EncodableCrateName(pub String);

impl<'de> Deserialize<'de> for EncodableCrateName {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<EncodableCrateName, D::Error> {
        let s = String::deserialize(d)?;
        if !Crate::valid_name(&s) {
            let value = de::Unexpected::Str(&s);
            let expected = format!(
                "a valid crate name to start with a letter, contain only letters, \
                 numbers, hyphens, or underscores and have at most {MAX_NAME_LENGTH} characters"
            );
            Err(de::Error::invalid_value(value, &expected.as_ref()))
        } else {
            Ok(EncodableCrateName(s))
        }
    }
}

#[derive(Serialize, Clone, Debug, Deref)]
pub struct EncodableDependencyName(pub String);

impl<'de> Deserialize<'de> for EncodableDependencyName {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<EncodableDependencyName, D::Error> {
        let s = String::deserialize(d)?;
        if !Crate::valid_dependency_name(&s) {
            let value = de::Unexpected::Str(&s);
            let expected = format!(
                "a valid dependency name to start with a letter or underscore, contain only letters, \
                 numbers, hyphens, or underscores and have at most {MAX_NAME_LENGTH} characters"
            );
            Err(de::Error::invalid_value(value, &expected.as_ref()))
        } else {
            Ok(EncodableDependencyName(s))
        }
    }
}

#[derive(Serialize, Clone, Debug, Deref)]
pub struct EncodableFeature(pub String);

impl<'de> Deserialize<'de> for EncodableFeature {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<EncodableFeature, D::Error> {
        let s = String::deserialize(d)?;
        if !Crate::valid_feature(&s) {
            let value = de::Unexpected::Str(&s);
            let expected = "a valid feature name";
            Err(de::Error::invalid_value(value, &expected))
        } else {
            Ok(EncodableFeature(s))
        }
    }
}

impl ToSql<Text, Pg> for EncodableFeature {
    fn to_sql(&self, out: &mut Output<'_, '_, Pg>) -> serialize::Result {
        ToSql::<Text, Pg>::to_sql(&**self, &mut out.reborrow())
    }
}

#[derive(Serialize, Debug, Deref)]
pub struct EncodableCrateVersion(pub semver::Version);

impl<'de> Deserialize<'de> for EncodableCrateVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<EncodableCrateVersion, D::Error> {
        let s = String::deserialize(d)?;
        match semver::Version::parse(&s) {
            Ok(v) => Ok(EncodableCrateVersion(v)),
            Err(..) => {
                let value = de::Unexpected::Str(&s);
                let expected = "a valid semver";
                Err(de::Error::invalid_value(value, &expected))
            }
        }
    }
}

#[derive(Serialize, Clone, Debug, Deref)]
pub struct EncodableCrateVersionReq(pub String);

impl<'de> Deserialize<'de> for EncodableCrateVersionReq {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<EncodableCrateVersionReq, D::Error> {
        let s = String::deserialize(d)?;
        match semver::VersionReq::parse(&s) {
            Ok(_) => Ok(EncodableCrateVersionReq(s)),
            Err(..) => {
                let value = de::Unexpected::Str(&s);
                let expected = "a valid version req";
                Err(de::Error::invalid_value(value, &expected))
            }
        }
    }
}

#[test]
fn feature_deserializes_for_valid_features() {
    use serde_json as json;

    assert_ok!(json::from_str::<EncodableFeature>("\"foo\""));
    assert_err!(json::from_str::<EncodableFeature>("\"\""));
    assert_err!(json::from_str::<EncodableFeature>("\"/\""));
    assert_err!(json::from_str::<EncodableFeature>("\"%/%\""));
    assert_ok!(json::from_str::<EncodableFeature>("\"a/a\""));
    assert_ok!(json::from_str::<EncodableFeature>("\"32-column-tables\""));
}
