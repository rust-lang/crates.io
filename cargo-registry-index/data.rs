use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::Write;

#[derive(Serialize, Deserialize, Debug)]
pub struct Crate {
    pub name: String,
    pub vers: String,
    pub deps: Vec<Dependency>,
    pub cksum: String,
    pub features: BTreeMap<String, Vec<String>>,
    /// This field contains features with new, extended syntax. Specifically,
    /// namespaced features (`dep:`) and weak dependencies (`pkg?/feat`).
    ///
    /// It is only populated if a feature uses the new syntax. Cargo merges it
    /// on top of the `features` field when reading the entries.
    ///
    /// This is separated from `features` because versions older than 1.19
    /// will fail to load due to not being able to parse the new syntax, even
    /// with a `Cargo.lock` file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features2: Option<BTreeMap<String, Vec<String>>>,
    pub yanked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_version: Option<String>,
    /// The schema version for this entry.
    ///
    /// If this is None, it defaults to version 1. Entries with unknown
    /// versions are ignored by cargo starting with 1.51.
    ///
    /// Version `2` format adds the `features2` field.
    ///
    /// This provides a method to safely introduce changes to index entries
    /// and allow older versions of cargo to ignore newer entries it doesn't
    /// understand. This is honored as of 1.51, so unfortunately older
    /// versions will ignore it, and potentially misinterpret version 2 and
    /// newer entries.
    ///
    /// The intent is that versions older than 1.51 will work with a
    /// pre-existing `Cargo.lock`, but they may not correctly process `cargo
    /// update` or build a lock from scratch. In that case, cargo may
    /// incorrectly select a new package that uses a new index format. A
    /// workaround is to downgrade any packages that are incompatible with the
    /// `--precise` flag of `cargo update`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v: Option<u32>,
}

fn write_crate<W: Write>(krate: &Crate, mut writer: W) -> anyhow::Result<()> {
    serde_json::to_writer(&mut writer, krate)?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn write_crates<W: Write>(crates: &[Crate], mut writer: W) -> anyhow::Result<()> {
    for krate in crates {
        write_crate(krate, &mut writer)?;
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: Option<DependencyKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
}

impl PartialOrd for Dependency {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Dependency {
    fn cmp(&self, other: &Self) -> Ordering {
        // In old `cargo` versions the dependency order appears to matter if the
        // same dependency exists twice but with different `kind` fields. In
        // those cases the `optional` field can sometimes be ignored or
        // misinterpreted. With this manual `Ord` implementation we ensure that
        // `normal` dependencies are always first when multiple with the same
        // `name` exist.
        (
            &self.name,
            self.kind,
            &self.req,
            self.optional,
            self.default_features,
            &self.target,
            &self.package,
            &self.features,
        )
            .cmp(&(
                &other.name,
                other.kind,
                &other.req,
                other.optional,
                other.default_features,
                &other.target,
                &other.package,
                &other.features,
            ))
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, PartialOrd, Ord, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Normal,
    Build,
    Dev,
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::*;

    #[test]
    fn crate_writer() {
        let krate = Crate {
            name: "foo".to_string(),
            vers: "1.2.3".to_string(),
            deps: vec![],
            cksum: "0123456789asbcdef".to_string(),
            features: Default::default(),
            features2: None,
            yanked: None,
            links: None,
            rust_version: None,
            v: None,
        };
        let mut buffer = Vec::new();
        assert_ok!(write_crate(&krate, &mut buffer));
        assert_ok_eq!(String::from_utf8(buffer), "\
            {\"name\":\"foo\",\"vers\":\"1.2.3\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
        ");
    }

    #[test]
    fn test_write_crates() {
        let versions = vec!["0.1.0", "1.0.0-beta.1", "1.0.0", "1.2.3"];
        let crates = versions
            .into_iter()
            .map(|vers| Crate {
                name: "foo".to_string(),
                vers: vers.to_string(),
                deps: vec![],
                cksum: "0123456789asbcdef".to_string(),
                features: Default::default(),
                features2: None,
                yanked: None,
                links: None,
                rust_version: None,
                v: None,
            })
            .collect::<Vec<_>>();

        let mut buffer = Vec::new();
        assert_ok!(write_crates(&crates, &mut buffer));
        assert_ok_eq!(String::from_utf8(buffer), "\
            {\"name\":\"foo\",\"vers\":\"0.1.0\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
            {\"name\":\"foo\",\"vers\":\"1.0.0-beta.1\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
            {\"name\":\"foo\",\"vers\":\"1.0.0\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
            {\"name\":\"foo\",\"vers\":\"1.2.3\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
        ");
    }
}
