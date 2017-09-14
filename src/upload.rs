//! This module handles the expected information a crate should have
//! and manages the serialising and deserialising of this information
//! to and from structs. The serlializing is only utilised in
//! integration tests.
use std::collections::HashMap;
use std::ops::Deref;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use semver;
use dependency::Kind as DependencyKind;

use keyword::Keyword as CrateKeyword;
use krate::{Crate, MAX_NAME_LENGTH};

#[derive(Deserialize, Serialize, Debug)]
pub struct NewCrate {
    pub name: CrateName,
    pub vers: CrateVersion,
    pub deps: Vec<CrateDependency>,
    pub features: HashMap<CrateName, Vec<Feature>>,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub readme: Option<String>,
    pub keywords: Option<KeywordList>,
    pub categories: Option<CategoryList>,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub repository: Option<String>,
    pub badges: Option<HashMap<String, HashMap<String, String>>>,
}

#[derive(PartialEq, Eq, Hash, Serialize, Debug)]
pub struct CrateName(pub String);
#[derive(Debug)]
pub struct CrateVersion(pub semver::Version);
#[derive(Debug)]
pub struct CrateVersionReq(pub semver::VersionReq);
#[derive(Serialize, Debug)]
pub struct KeywordList(pub Vec<Keyword>);
#[derive(Serialize, Debug)]
pub struct Keyword(pub String);
#[derive(Serialize, Debug)]
pub struct CategoryList(pub Vec<Category>);
#[derive(Serialize, Deserialize, Debug)]
pub struct Category(pub String);
#[derive(Serialize, Debug)]
pub struct Feature(pub String);

#[derive(Serialize, Deserialize, Debug)]
pub struct CrateDependency {
    pub optional: bool,
    pub default_features: bool,
    pub name: CrateName,
    pub features: Vec<Feature>,
    pub version_req: CrateVersionReq,
    pub target: Option<String>,
    pub kind: Option<DependencyKind>,
}

impl<'de> Deserialize<'de> for CrateName {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<CrateName, D::Error> {
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
            Ok(CrateName(s))
        }
    }
}

impl<T: ?Sized> PartialEq<T> for CrateName
where
    String: PartialEq<T>,
{
    fn eq(&self, rhs: &T) -> bool {
        self.0 == *rhs
    }
}

impl<'de> Deserialize<'de> for Keyword {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Keyword, D::Error> {
        let s = String::deserialize(d)?;
        if !CrateKeyword::valid_name(&s) {
            let value = de::Unexpected::Str(&s);
            let expected = "a valid keyword specifier";
            Err(de::Error::invalid_value(value, &expected))
        } else {
            Ok(Keyword(s))
        }
    }
}

impl<'de> Deserialize<'de> for Feature {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Feature, D::Error> {
        let s = String::deserialize(d)?;
        if !Crate::valid_feature_name(&s) {
            let value = de::Unexpected::Str(&s);
            let expected = "a valid feature name";
            Err(de::Error::invalid_value(value, &expected))
        } else {
            Ok(Feature(s))
        }
    }
}

impl<'de> Deserialize<'de> for CrateVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<CrateVersion, D::Error> {
        let s = String::deserialize(d)?;
        match semver::Version::parse(&s) {
            Ok(v) => Ok(CrateVersion(v)),
            Err(..) => {
                let value = de::Unexpected::Str(&s);
                let expected = "a valid semver";
                Err(de::Error::invalid_value(value, &expected))
            }
        }
    }
}

impl<'de> Deserialize<'de> for CrateVersionReq {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<CrateVersionReq, D::Error> {
        let s = String::deserialize(d)?;
        match semver::VersionReq::parse(&s) {
            Ok(v) => Ok(CrateVersionReq(v)),
            Err(..) => {
                let value = de::Unexpected::Str(&s);
                let expected = "a valid version req";
                Err(de::Error::invalid_value(value, &expected))
            }
        }
    }
}

impl<T: ?Sized> PartialEq<T> for CrateVersionReq
where
    semver::VersionReq: PartialEq<T>,
{
    fn eq(&self, rhs: &T) -> bool {
        self.0 == *rhs
    }
}

impl<'de> Deserialize<'de> for KeywordList {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<KeywordList, D::Error> {
        let inner = <Vec<Keyword> as Deserialize<'de>>::deserialize(d)?;
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
        Ok(KeywordList(inner))
    }
}

impl<'de> Deserialize<'de> for CategoryList {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<CategoryList, D::Error> {
        let inner = <Vec<Category> as Deserialize<'de>>::deserialize(d)?;
        if inner.len() > 5 {
            let expected = "at most 5 categories per crate";
            Err(de::Error::invalid_length(inner.len(), &expected))
        } else {
            Ok(CategoryList(inner))
        }
    }
}
impl Serialize for CrateVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&(**self).to_string())
    }
}

impl Serialize for CrateVersionReq {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&(**self).to_string())
    }
}

impl Deref for CrateName {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl Deref for Keyword {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl Deref for Category {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl Deref for Feature {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl Deref for CrateVersion {
    type Target = semver::Version;
    fn deref(&self) -> &semver::Version {
        &self.0
    }
}

impl Deref for CrateVersionReq {
    type Target = semver::VersionReq;
    fn deref(&self) -> &semver::VersionReq {
        &self.0
    }
}

impl Deref for KeywordList {
    type Target = [Keyword];
    fn deref(&self) -> &[Keyword] {
        &self.0
    }
}

impl Deref for CategoryList {
    type Target = [Category];
    fn deref(&self) -> &[Category] {
        &self.0
    }
}
