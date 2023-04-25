use std::cmp;

pub use self::bytes_request::BytesRequest;
pub use self::io_util::{read_fill, read_le_u32, LimitErrorReader};
pub use self::request_helpers::*;

mod bytes_request;
pub mod errors;
mod io_util;
pub mod manifest;
mod request_helpers;
pub mod rfc3339;
pub mod token;
pub mod tracing;

#[derive(Debug, Copy, Clone)]
pub struct Maximums {
    pub max_upload_size: u64,
    pub max_unpack_size: u64,
}

impl Maximums {
    pub fn new(
        krate_max_upload: Option<i32>,
        app_max_upload: u64,
        app_max_unpack: u64,
    ) -> Maximums {
        let max_upload_size = krate_max_upload.map(|m| m as u64).unwrap_or(app_max_upload);
        let max_unpack_size = cmp::max(app_max_unpack, max_upload_size);
        Maximums {
            max_upload_size,
            max_unpack_size,
        }
    }
}

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
