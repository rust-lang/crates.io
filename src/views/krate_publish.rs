//! This module handles the expected information a crate should have
//! and manages the serialising and deserialising of this information
//! to and from structs. The serlializing is only utilised in
//! integration tests.
use std::collections::HashMap;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::models::krate::MAX_NAME_LENGTH;

use crate::models::Crate;
use crate::models::DependencyKind;
use crate::models::Keyword as CrateKeyword;

#[derive(Deserialize, Serialize, Debug)]
pub struct EncodableCrateUpload {
    pub name: EncodableCrateName,
    pub vers: EncodableCrateVersion,
    pub deps: Vec<EncodableCrateDependency>,
    pub features: HashMap<EncodableFeatureName, Vec<EncodableFeature>>,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub readme: Option<String>,
    pub readme_file: Option<String>,
    #[serde(default)]
    pub keywords: EncodableKeywordList,
    #[serde(default)]
    pub categories: EncodableCategoryList,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub repository: Option<String>,
    pub badges: Option<HashMap<String, HashMap<String, String>>>,
    #[serde(default)]
    pub links: Option<String>,
}

#[derive(PartialEq, Eq, Hash, Serialize, Debug, Deref)]
pub struct EncodableCrateName(pub String);
#[derive(Debug, Deref)]
pub struct EncodableCrateVersion(pub semver::Version);
#[derive(Debug, Deref)]
pub struct EncodableCrateVersionReq(pub semver::VersionReq);
#[derive(Serialize, Debug, Deref, Default)]
pub struct EncodableKeywordList(pub Vec<EncodableKeyword>);
#[derive(Serialize, Debug, Deref)]
pub struct EncodableKeyword(pub String);
#[derive(Serialize, Debug, Deref, Default)]
pub struct EncodableCategoryList(pub Vec<EncodableCategory>);
#[derive(Serialize, Deserialize, Debug, Deref)]
pub struct EncodableCategory(pub String);
#[derive(Serialize, Debug, Deref)]
pub struct EncodableFeature(pub String);
#[derive(PartialEq, Eq, Hash, Serialize, Debug, Deref)]
pub struct EncodableFeatureName(pub String);

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableCrateDependency {
    pub optional: bool,
    pub default_features: bool,
    pub name: EncodableCrateName,
    pub features: Vec<EncodableFeature>,
    pub version_req: EncodableCrateVersionReq,
    pub target: Option<String>,
    pub kind: Option<DependencyKind>,
    pub explicit_name_in_toml: Option<EncodableCrateName>,
    pub registry: Option<String>,
}

impl<'de> Deserialize<'de> for EncodableCrateName {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<EncodableCrateName, D::Error> {
        let s = String::deserialize(d)?;
        if !Crate::valid_name(&s) {
            let value = de::Unexpected::Str(&s);
            let expected = format!(
                "a valid crate name to start with a letter, contain only letters, \
                 numbers, hyphens, or underscores and have at most {} characters",
                MAX_NAME_LENGTH
            );
            Err(de::Error::invalid_value(value, &expected.as_ref()))
        } else {
            Ok(EncodableCrateName(s))
        }
    }
}

impl<T: ?Sized> PartialEq<T> for EncodableCrateName
where
    String: PartialEq<T>,
{
    fn eq(&self, rhs: &T) -> bool {
        self.0 == *rhs
    }
}

impl<'de> Deserialize<'de> for EncodableKeyword {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<EncodableKeyword, D::Error> {
        let s = String::deserialize(d)?;
        if !CrateKeyword::valid_name(&s) {
            let value = de::Unexpected::Str(&s);
            let expected = "a valid keyword specifier";
            Err(de::Error::invalid_value(value, &expected))
        } else {
            Ok(EncodableKeyword(s))
        }
    }
}

impl<'de> Deserialize<'de> for EncodableFeatureName {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        if !Crate::valid_feature_name(&s) {
            let value = de::Unexpected::Str(&s);
            let expected = "a valid feature name containing only letters, \
                            numbers, hyphens, or underscores";
            Err(de::Error::invalid_value(value, &expected))
        } else {
            Ok(EncodableFeatureName(s))
        }
    }
}

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

impl<'de> Deserialize<'de> for EncodableCrateVersionReq {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<EncodableCrateVersionReq, D::Error> {
        let s = String::deserialize(d)?;
        match semver::VersionReq::parse(&s) {
            Ok(v) => Ok(EncodableCrateVersionReq(v)),
            Err(..) => {
                let value = de::Unexpected::Str(&s);
                let expected = "a valid version req";
                Err(de::Error::invalid_value(value, &expected))
            }
        }
    }
}

impl<T: ?Sized> PartialEq<T> for EncodableCrateVersionReq
where
    semver::VersionReq: PartialEq<T>,
{
    fn eq(&self, rhs: &T) -> bool {
        self.0 == *rhs
    }
}

impl<'de> Deserialize<'de> for EncodableKeywordList {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<EncodableKeywordList, D::Error> {
        let inner = <Vec<EncodableKeyword> as Deserialize<'de>>::deserialize(d)?;
        if inner.len() > 5 {
            let expected = "at most 5 keywords per crate";
            return Err(de::Error::invalid_length(inner.len(), &expected));
        }
        for val in &inner {
            if val.len() > 20 {
                let expected = "a keyword with less than 20 characters";
                return Err(de::Error::invalid_length(val.len(), &expected));
            }
        }
        Ok(EncodableKeywordList(inner))
    }
}

impl<'de> Deserialize<'de> for EncodableCategoryList {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<EncodableCategoryList, D::Error> {
        let inner = <Vec<EncodableCategory> as Deserialize<'de>>::deserialize(d)?;
        if inner.len() > 5 {
            let expected = "at most 5 categories per crate";
            Err(de::Error::invalid_length(inner.len(), &expected))
        } else {
            Ok(EncodableCategoryList(inner))
        }
    }
}

impl Serialize for EncodableCrateVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&(**self).to_string())
    }
}

impl Serialize for EncodableCrateVersionReq {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&(**self).to_string())
    }
}

use diesel::pg::Pg;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use std::io::Write;

impl ToSql<Text, Pg> for EncodableFeature {
    fn to_sql<W: Write>(&self, out: &mut Output<'_, W, Pg>) -> serialize::Result {
        ToSql::<Text, Pg>::to_sql(&**self, out)
    }
}

#[test]
fn feature_deserializes_for_valid_features() {
    use serde_json as json;

    assert!(json::from_str::<EncodableFeature>("\"foo\"").is_ok());
    assert!(json::from_str::<EncodableFeature>("\"\"").is_err());
    assert!(json::from_str::<EncodableFeature>("\"/\"").is_err());
    assert!(json::from_str::<EncodableFeature>("\"%/%\"").is_err());
    assert!(json::from_str::<EncodableFeature>("\"a/a\"").is_ok());
    assert!(json::from_str::<EncodableFeature>("\"32-column-tables\"").is_ok());
}
