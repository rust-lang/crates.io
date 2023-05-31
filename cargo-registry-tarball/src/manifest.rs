use derive_deref::Deref;
use serde::{de, Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct Manifest {
    #[serde(alias = "project")]
    pub package: Package,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Package {
    pub readme: Option<String>,
    pub repository: Option<String>,
    pub rust_version: Option<RustVersion>,
}

#[derive(Debug, Deref)]
pub struct RustVersion(String);

impl PartialEq<&str> for RustVersion {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq(other)
    }
}

impl<'de> Deserialize<'de> for RustVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<RustVersion, D::Error> {
        let s = String::deserialize(d)?;
        match semver::VersionReq::parse(&s) {
            // Exclude semver operators like `^` and pre-release identifiers
            Ok(_) if s.chars().all(|c| c.is_ascii_digit() || c == '.') => Ok(RustVersion(s)),
            Ok(_) | Err(..) => {
                let value = de::Unexpected::Str(&s);
                let expected = "a valid rust_version";
                Err(de::Error::invalid_value(value, &expected))
            }
        }
    }
}
