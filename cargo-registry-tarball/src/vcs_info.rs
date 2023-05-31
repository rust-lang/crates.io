use serde::Deserialize;

/// Represents relevant contents of .cargo_vcs_info.json file when uploaded from cargo
/// or downloaded from crates.io
#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct CargoVcsInfo {
    /// Path to the package within repo (empty string if root). / not \
    #[serde(default)]
    pub path_in_vcs: String,
}

impl CargoVcsInfo {
    pub fn from_contents(contents: &str) -> serde_json::Result<Self> {
        serde_json::from_str(contents)
    }
}

#[cfg(test)]
mod tests {
    use super::CargoVcsInfo;

    #[test]
    fn test_cargo_vcs_info() {
        assert_eq!(CargoVcsInfo::from_contents("").ok(), None);
        assert_eq!(
            CargoVcsInfo::from_contents("{}").unwrap(),
            CargoVcsInfo {
                path_in_vcs: "".into()
            }
        );
        assert_eq!(
            CargoVcsInfo::from_contents(r#"{"path_in_vcs": "hi"}"#).unwrap(),
            CargoVcsInfo {
                path_in_vcs: "hi".into()
            }
        );
        assert_eq!(
            CargoVcsInfo::from_contents(r#"{"path_in_vcs": "hi", "future": "field"}"#).unwrap(),
            CargoVcsInfo {
                path_in_vcs: "hi".into()
            }
        );
    }
}
