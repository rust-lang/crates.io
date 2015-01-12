use std::collections::HashMap;
use std::ops::Deref;

use rustc_serialize::{Decodable, Decoder, Encoder, Encodable};
use semver;
use dependency::Kind as DependencyKind;

use keyword::Keyword as CrateKeyword;
use krate::Crate;

#[derive(RustcDecodable, RustcEncodable)]
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
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub repository: Option<String>,
}

#[derive(PartialEq, Eq, Hash)]
pub struct CrateName(pub String);
pub struct CrateVersion(pub semver::Version);
pub struct CrateVersionReq(pub semver::VersionReq);
pub struct KeywordList(pub Vec<Keyword>);
pub struct Keyword(pub String);
pub struct Feature(pub String);

#[derive(RustcDecodable, RustcEncodable)]
pub struct CrateDependency {
    pub optional: bool,
    pub default_features: bool,
    pub name: CrateName,
    pub features: Vec<Feature>,
    pub version_req: CrateVersionReq,
    pub target: Option<String>,
    pub kind: Option<DependencyKind>,
}

impl Decodable for CrateName {
    fn decode<D: Decoder>(d: &mut D) -> Result<CrateName, D::Error> {
        let s = try!(d.read_str());
        if !Crate::valid_name(s.as_slice()) {
            return Err(d.error(format!("invalid crate name specified: {}",
                                       s).as_slice()))
        }
        Ok(CrateName(s))
    }
}

impl Decodable for Keyword {
    fn decode<D: Decoder>(d: &mut D) -> Result<Keyword, D::Error> {
        let s = try!(d.read_str());
        if !CrateKeyword::valid_name(s.as_slice()) {
            return Err(d.error(format!("invalid keyword specified: {}",
                                       s).as_slice()))
        }
        Ok(Keyword(s))
    }
}

impl Decodable for Feature {
    fn decode<D: Decoder>(d: &mut D) -> Result<Feature, D::Error> {
        let s = try!(d.read_str());
        if !Crate::valid_feature_name(s.as_slice()) {
            return Err(d.error(format!("invalid feature name specified: {}",
                                       s).as_slice()))
        }
        Ok(Feature(s))
    }
}

impl Decodable for CrateVersion {
    fn decode<D: Decoder>(d: &mut D) -> Result<CrateVersion, D::Error> {
        let s = try!(d.read_str());
        match semver::Version::parse(s.as_slice()) {
            Ok(v) => Ok(CrateVersion(v)),
            Err(..) => Err(d.error(format!("invalid semver: {}", s).as_slice())),
        }
    }
}

impl Decodable for CrateVersionReq {
    fn decode<D: Decoder>(d: &mut D) -> Result<CrateVersionReq, D::Error> {
        let s = try!(d.read_str());
        match semver::VersionReq::parse(s.as_slice()) {
            Ok(v) => Ok(CrateVersionReq(v)),
            Err(..) => Err(d.error(format!("invalid version req: {}",
                                           s).as_slice())),
        }
    }
}

impl Decodable for KeywordList {
    fn decode<D: Decoder>(d: &mut D) -> Result<KeywordList, D::Error> {
        let inner: Vec<Keyword> = try!(Decodable::decode(d));
        if inner.len() > 5 {
            return Err(d.error("a maximum of 5 keywords per crate are allowed"))
        }
        for val in inner.iter() {
            if val.len() > 20 {
                return Err(d.error("keywords must contain less than 20 \
                                    characters"))
            }
        }
        Ok(KeywordList(inner))
    }
}

impl Decodable for DependencyKind {
    fn decode<D: Decoder>(d: &mut D) -> Result<DependencyKind, D::Error> {
        let s: String = try!(Decodable::decode(d));
        match s.as_slice() {
            "dev" => Ok(DependencyKind::Dev),
            "build" => Ok(DependencyKind::Build),
            "normal" => Ok(DependencyKind::Normal),
            s => Err(d.error(format!("invalid dependency kind `{}`, must be \
                                      one of dev, build, or normal",
                                     s).as_slice())),
        }
    }
}

impl Encodable for CrateName {
    fn encode<E: Encoder>(&self, d: &mut E) -> Result<(), E::Error> {
        d.emit_str(self.as_slice())
    }
}

impl Encodable for Keyword {
    fn encode<E: Encoder>(&self, d: &mut E) -> Result<(), E::Error> {
        d.emit_str(self.as_slice())
    }
}

impl Encodable for Feature {
    fn encode<E: Encoder>(&self, d: &mut E) -> Result<(), E::Error> {
        d.emit_str(self.as_slice())
    }
}

impl Encodable for CrateVersion {
    fn encode<E: Encoder>(&self, d: &mut E) -> Result<(), E::Error> {
        d.emit_str((**self).to_string().as_slice())
    }
}

impl Encodable for CrateVersionReq {
    fn encode<E: Encoder>(&self, d: &mut E) -> Result<(), E::Error> {
        d.emit_str((**self).to_string().as_slice())
    }
}

impl Encodable for KeywordList {
    fn encode<E: Encoder>(&self, d: &mut E) -> Result<(), E::Error> {
        let KeywordList(ref inner) = *self;
        inner.encode(d)
    }
}

impl Encodable for DependencyKind {
    fn encode<E: Encoder>(&self, d: &mut E) -> Result<(), E::Error> {
        match *self {
            DependencyKind::Normal => "normal".encode(d),
            DependencyKind::Build => "build".encode(d),
            DependencyKind::Dev => "dev".encode(d),
        }
    }
}

impl Deref for CrateName {
    type Target = str;
    fn deref<'a>(&'a self) -> &'a str {
        let CrateName(ref s) = *self;
        s.as_slice()
    }
}

impl Deref for Keyword {
    type Target = str;
    fn deref<'a>(&'a self) -> &'a str {
        let Keyword(ref s) = *self;
        s.as_slice()
    }
}

impl Deref for Feature {
    type Target = str;
    fn deref<'a>(&'a self) -> &'a str {
        let Feature(ref s) = *self;
        s.as_slice()
    }
}

impl Deref for CrateVersion {
    type Target = semver::Version;
    fn deref<'a>(&'a self) -> &'a semver::Version {
        let CrateVersion(ref s) = *self; s
    }
}

impl Deref for CrateVersionReq {
    type Target = semver::VersionReq;
    fn deref<'a>(&'a self) -> &'a semver::VersionReq {
        let CrateVersionReq(ref s) = *self; s
    }
}

impl Deref for KeywordList {
    type Target = [Keyword];
    fn deref<'a>(&'a self) -> &'a [Keyword] {
        let KeywordList(ref s) = *self;
        s.as_slice()
    }
}
