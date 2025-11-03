/// Extracts the workflow path from a GitLab `ci_config_ref_uri` claim.
///
/// In other words, it turns e.g. `gitlab.com/rust-lang/regex//foo/bar/baz.yml@refs/heads/main`
/// into `foo/bar/baz.yml`, or `None` if the reference is in an unexpected format.
///
/// This was initially using a regular expression (`//(.*[^/]\.(yml|yaml))@.+`),
/// but was changed to string operations to avoid potential ReDoS attack vectors
/// (see `test_extract_workflow_filename_redos` test below).
pub(crate) fn extract_workflow_filepath(workflow_ref: &str) -> Option<&str> {
    // Find the double slash that separates project path from workflow path
    let start = workflow_ref.find("//")?;
    let after_double_slash = &workflow_ref[start + 2..];

    // Find the last @ that separates workflow path from ref
    let end = after_double_slash.rfind('@')?;
    let filepath = &after_double_slash[..end];

    // Validate: must end with .yml or .yaml
    if !filepath.ends_with(".yml") && !filepath.ends_with(".yaml") {
        return None;
    }

    // Get the basename (part after last slash, or whole string if no slash)
    let basename = filepath.rsplit('/').next()?;

    // Basename must not be empty aside from extension (rejects ".yml", ".yaml", "somedir/.yaml")
    if basename == ".yml" || basename == ".yaml" {
        return None;
    }

    Some(filepath)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_extract_workflow_filename() {
        let test_cases = [
            // Well-formed `ci_config_ref_uri`s, including obnoxious ones.
            (
                "gitlab.com/foo/bar//notnested.yml@/some/ref",
                Some("notnested.yml"),
            ),
            (
                "gitlab.com/foo/bar//notnested.yaml@/some/ref",
                Some("notnested.yaml"),
            ),
            (
                "gitlab.com/foo/bar//basic/basic.yml@/some/ref",
                Some("basic/basic.yml"),
            ),
            (
                "gitlab.com/foo/bar//more/nested/example.yml@/some/ref",
                Some("more/nested/example.yml"),
            ),
            (
                "gitlab.com/foo/bar//too//many//slashes.yml@/some/ref",
                Some("too//many//slashes.yml"),
            ),
            ("gitlab.com/foo/bar//has-@.yml@/some/ref", Some("has-@.yml")),
            (
                "gitlab.com/foo/bar//foo.bar.yml@/some/ref",
                Some("foo.bar.yml"),
            ),
            (
                "gitlab.com/foo/bar//foo.yml.bar.yml@/some/ref",
                Some("foo.yml.bar.yml"),
            ),
            (
                "gitlab.com/foo/bar//foo.yml@bar.yml@/some/ref",
                Some("foo.yml@bar.yml"),
            ),
            (
                "gitlab.com/foo/bar//@foo.yml@bar.yml@/some/ref",
                Some("@foo.yml@bar.yml"),
            ),
            (
                "gitlab.com/foo/bar//@.yml.foo.yml@bar.yml@/some/ref",
                Some("@.yml.foo.yml@bar.yml"),
            ),
            ("gitlab.com/foo/bar//a.yml@refs/heads/main", Some("a.yml")),
            (
                "gitlab.com/foo/bar//a/b.yml@refs/heads/main",
                Some("a/b.yml"),
            ),
            (
                "gitlab.com/foo/bar//.gitlab-ci.yml@refs/heads/main",
                Some(".gitlab-ci.yml"),
            ),
            (
                "gitlab.com/foo/bar//.gitlab-ci.yaml@refs/heads/main",
                Some(".gitlab-ci.yaml"),
            ),
            // Malformed `ci_config_ref_uri`s.
            ("gitlab.com/foo/bar//notnested.wrongsuffix@/some/ref", None),
            ("gitlab.com/foo/bar//@/some/ref", None),
            ("gitlab.com/foo/bar//.yml@/some/ref", None),
            ("gitlab.com/foo/bar//.yaml@/some/ref", None),
            ("gitlab.com/foo/bar//somedir/.yaml@/some/ref", None),
        ];

        for (input, expected) in test_cases {
            let result = super::extract_workflow_filepath(input);
            assert_eq!(result, expected, "Input: {input}");
        }
    }

    #[test]
    fn test_extract_workflow_filename_redos() {
        let _ = super::extract_workflow_filepath(
            &(".yml@//".repeat(200_000_000) + ".yml@/\n//\x00.yml@y"),
        );
    }
}
