use semver::Version;
use tracing::instrument;

/// Parse crate name and version from a download URL or URL path.
///
/// This function supports both URL formats:
///
/// - `https://static.crates.io/crates/foo/foo-1.2.3.crate`
/// - `https://static.crates.io/crates/foo/1.2.3/download`
#[instrument(level = "debug")]
pub fn parse_path(mut path: &str) -> Option<(String, Version)> {
    // This would ideally use a regular expression to simplify the code, but
    // regexes are slow, and we want to keep this code as fast as possible.

    // Remove any query parameters.
    if let Some(pos) = path.find('?') {
        path = &path[..pos];
    }

    // Find the start of the path. We assume that we don't have any nested
    // `crates` folders on the server (e.g. `/foo/crates/...`).
    let pos = path.find("/crates/")?;
    let path = &path[pos + 8..];

    // The following code supports both `foo/1.2.3/download`
    // and `foo/foo-1.2.3.crate`
    let (folder, rest) = path.split_once('/')?;
    let version = rest.strip_suffix("/download").or_else(|| {
        rest.strip_suffix(".crate")
            .and_then(|rest| rest.strip_prefix(folder))
            .and_then(|rest| rest.strip_prefix('-'))
    })?;

    let version = Version::parse(version).ok()?;

    Some((folder.to_owned(), version))
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some};
    use semver::Version;

    fn format((name, version): &(String, Version)) -> String {
        format!("{name}@{version}")
    }

    #[test]
    fn test_parse_path_valid() {
        let result = assert_some!(parse_path("/crates/foo/foo-1.2.3.crate"));
        assert_eq!(format(&result), "foo@1.2.3");

        let result = assert_some!(parse_path("/crates/foo/1.2.3/download"));
        assert_eq!(format(&result), "foo@1.2.3");
    }

    #[test]
    fn test_parse_path_with_query_params() {
        let result = assert_some!(parse_path("/crates/foo/foo-1.2.3.crate?param=value"));
        assert_eq!(format(&result), "foo@1.2.3");

        let result = assert_some!(parse_path("/crates/foo/1.2.3/download"));
        assert_eq!(format(&result), "foo@1.2.3");
    }

    #[test]
    fn test_parse_path_with_full_url() {
        let path = "https://static.crates.io/crates/foo/foo-1.2.3.crate";
        let result = assert_some!(parse_path(path));
        assert_eq!(format(&result), "foo@1.2.3");

        let path = "https://static.crates.io/crates/foo/1.2.3/download";
        let result = assert_some!(parse_path(path));
        assert_eq!(format(&result), "foo@1.2.3");
    }

    #[test]
    fn test_parse_path_with_dashes() {
        let path = "/crates/foo-bar/foo-bar-1.0.0-rc.1.crate";
        let result = assert_some!(parse_path(path));
        assert_eq!(format(&result), "foo-bar@1.0.0-rc.1");

        let path = "/crates/foo-bar/1.0.0-rc.1/download";
        let result = assert_some!(parse_path(path));
        assert_eq!(format(&result), "foo-bar@1.0.0-rc.1");
    }

    #[test]
    fn test_parse_path_empty() {
        assert_none!(parse_path(""));
    }

    #[test]
    fn test_parse_path_only_query_params() {
        assert_none!(parse_path("?param=value"));
    }

    #[test]
    fn test_parse_path_only_crates_prefix() {
        assert_none!(parse_path("/crates/"));
    }

    #[test]
    fn test_parse_path_unrelated_path() {
        assert_none!(parse_path("/readmes/foo/foo-1.2.3.crate"));
    }

    #[test]
    fn test_parse_path_no_folder() {
        assert_none!(parse_path("/crates/foo-1.2.3.crate"));
    }

    #[test]
    fn test_parse_path_no_file_extension() {
        assert_none!(parse_path("/crates/foo/foo-1.2.3"));
    }

    #[test]
    fn test_parse_path_wrong_file_extension() {
        assert_none!(parse_path("/crates/foo/foo-1.2.3.html"));
    }

    #[test]
    fn test_parse_path_bad_crate_name() {
        assert_none!(parse_path("/crates/foo/bar-1.2.3.crate"));
    }

    #[test]
    fn test_parse_path_invalid_separator() {
        assert_none!(parse_path("/crates/foo/foo@1.2.3.crate"));
    }

    #[test]
    fn test_parse_path_no_version() {
        assert_none!(parse_path("/crates/foo/foo.crate"));
    }

    #[test]
    fn test_parse_path_invalid_version() {
        assert_none!(parse_path("/crates/foo/foo-1.2.3§foo.crate"));
    }
}
