use std::sync::LazyLock;

/// Extracts the workflow filename from a GitHub workflow reference.
///
/// In other words, it turns e.g. `rust-lang/regex/.github/workflows/ci.yml@refs/heads/main`
/// into `ci.yml`, or `None` if the reference is in an unexpected format.
#[allow(unused)]
pub(crate) fn extract_workflow_filename(workflow_ref: &str) -> Option<&str> {
    static WORKFLOW_REF_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"([^/]+\.(yml|yaml))(@.+)").unwrap());

    WORKFLOW_REF_RE
        .captures(workflow_ref)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_extract_workflow_filename() {
        let test_cases = [
            // Well-formed workflow refs, including exceedingly obnoxious ones
            // with `@` or extra suffixes or `git` refs that look like workflows.
            (
                "foo/bar/.github/workflows/basic.yml@refs/heads/main",
                Some("basic.yml"),
            ),
            (
                "foo/bar/.github/workflows/basic.yaml@refs/heads/main",
                Some("basic.yaml"),
            ),
            (
                "foo/bar/.github/workflows/has-dash.yml@refs/heads/main",
                Some("has-dash.yml"),
            ),
            (
                "foo/bar/.github/workflows/has--dashes.yml@refs/heads/main",
                Some("has--dashes.yml"),
            ),
            (
                "foo/bar/.github/workflows/has--dashes-.yml@refs/heads/main",
                Some("has--dashes-.yml"),
            ),
            (
                "foo/bar/.github/workflows/has.period.yml@refs/heads/main",
                Some("has.period.yml"),
            ),
            (
                "foo/bar/.github/workflows/has..periods.yml@refs/heads/main",
                Some("has..periods.yml"),
            ),
            (
                "foo/bar/.github/workflows/has..periods..yml@refs/heads/main",
                Some("has..periods..yml"),
            ),
            (
                "foo/bar/.github/workflows/has_underscore.yml@refs/heads/main",
                Some("has_underscore.yml"),
            ),
            (
                "foo/bar/.github/workflows/nested@evil.yml@refs/heads/main",
                Some("nested@evil.yml"),
            ),
            (
                "foo/bar/.github/workflows/nested.yml@evil.yml@refs/heads/main",
                Some("nested.yml@evil.yml"),
            ),
            (
                "foo/bar/.github/workflows/extra@nested.yml@evil.yml@refs/heads/main",
                Some("extra@nested.yml@evil.yml"),
            ),
            (
                "foo/bar/.github/workflows/extra.yml@nested.yml@evil.yml@refs/heads/main",
                Some("extra.yml@nested.yml@evil.yml"),
            ),
            (
                "foo/bar/.github/workflows/basic.yml@refs/heads/misleading@branch.yml",
                Some("basic.yml"),
            ),
            (
                "foo/bar/.github/workflows/basic.yml@refs/heads/bad@branch@twomatches.yml",
                Some("basic.yml"),
            ),
            (
                "foo/bar/.github/workflows/foo.yml.yml@refs/heads/main",
                Some("foo.yml.yml"),
            ),
            (
                "foo/bar/.github/workflows/foo.yml.foo.yml@refs/heads/main",
                Some("foo.yml.foo.yml"),
            ),
            // Malformed workflow refs.
            (
                "foo/bar/.github/workflows/basic.wrongsuffix@refs/heads/main",
                None,
            ),
            ("foo/bar/.github/workflows/@refs/heads/main", None),
            ("foo/bar/.github/workflows/nosuffix@refs/heads/main", None),
            ("foo/bar/.github/workflows/.yml@refs/heads/main", None),
            ("foo/bar/.github/workflows/.yaml@refs/heads/main", None),
            ("foo/bar/.github/workflows/main.yml", None),
        ];

        for (input, expected) in test_cases {
            let result = super::extract_workflow_filename(input);
            assert_eq!(result, expected, "Input: {input}");
        }
    }
}
