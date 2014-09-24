use std::fmt;
use serialize::{Encodable, Encoder};

use util::{CargoResult, human, Require};

pub struct Dependency {
    pub optional: bool,
    pub default_features: bool,
    pub name: String,
    pub features: Vec<String>,
    pub version_req: String,
}

impl Dependency {
    pub fn parse(s: &str) -> CargoResult<Dependency> {
        let mut parts = s.splitn(3, '|');
        let name = parts.next().unwrap();
        let features = try!(parts.next().require(|| {
            human(format!("malformed dependency: {}", name))
        }));
        let version_req = try!(parts.next().require(|| {
            human(format!("malformed dependency: {}", name))
        }));
        let (name, optional) = if name.starts_with("-") {
            (name.slice_from(1), true)
        } else {
            (name, false)
        };
        let (name, default_features) = if name.starts_with("*") {
            (name.slice_from(1), false)
        } else {
            (name, true)
        };
        Ok(Dependency {
            optional: optional,
            default_features: default_features,
            name: name.to_string(),
            features: features.split(',').map(|s| s.to_string()).collect(),
            version_req: version_req.to_string(),
        })
    }
}

impl fmt::Show for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}|{}|{}",
               if self.optional {"-"} else {""},
               if self.default_features {""} else {"*"},
               self.name,
               self.features.connect(","),
               self.version_req)
    }
}

impl<A, E: Encoder<A>> Encodable<E, A> for Dependency {
    fn encode(&self, into: &mut E) -> Result<(), A> {
        self.to_string().encode(into)
    }
}
