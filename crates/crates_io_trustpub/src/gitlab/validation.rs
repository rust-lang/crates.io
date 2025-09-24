//! Validation functions for GitLab Trusted Publishing configuration fields.
//!
//! This module performs basic validation of user input for GitLab CI/CD trusted publishing
//! configurations. The validation rules are intentionally permissive: they accept all valid
//! GitLab values while rejecting obviously invalid input. This approach is enough for our
//! purposes since GitLab's JWT claims will only contain valid values anyway.
//!
//! See <https://docs.gitlab.com/user/reserved_names/#rules-for-usernames-project-and-group-names-and-slugs>
//! and <https://docs.gitlab.com/ci/yaml/#environment>.

use std::sync::LazyLock;

const MAX_FIELD_LENGTH: usize = 255;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("GitLab namespace may not be empty")]
    NamespaceEmpty,
    #[error("GitLab namespace is too long (maximum is {MAX_FIELD_LENGTH} characters)")]
    NamespaceTooLong,
    #[error("Invalid GitLab namespace")]
    NamespaceInvalid,
    #[error("GitLab namespace cannot end with .atom or .git")]
    NamespaceInvalidSuffix,

    #[error("GitLab project name may not be empty")]
    ProjectEmpty,
    #[error("GitLab project name is too long (maximum is {MAX_FIELD_LENGTH} characters)")]
    ProjectTooLong,
    #[error("Invalid GitLab project name")]
    ProjectInvalid,
    #[error("GitLab project name cannot end with .atom or .git")]
    ProjectInvalidSuffix,

    #[error("Workflow filepath may not be empty")]
    WorkflowFilepathEmpty,
    #[error("Workflow filepath is too long (maximum is {MAX_FIELD_LENGTH} characters)")]
    WorkflowFilepathTooLong,
    #[error("Workflow filepath must end with `.yml` or `.yaml`")]
    WorkflowFilepathMissingSuffix,
    #[error("Workflow filepath cannot start with /")]
    WorkflowFilepathStartsWithSlash,
    #[error("Workflow filepath cannot end with /")]
    WorkflowFilepathEndsWithSlash,

    #[error("Environment name may not be empty (use `null` to omit)")]
    EnvironmentEmptyString,
    #[error("Environment name is too long (maximum is {MAX_FIELD_LENGTH} characters)")]
    EnvironmentTooLong,
    #[error("Environment name contains invalid characters")]
    EnvironmentInvalidChars,
}

pub fn validate_namespace(namespace: &str) -> Result<(), ValidationError> {
    static RE_VALID_NAMESPACE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"^[a-zA-Z0-9](?:[a-zA-Z0-9_.\-/]*[a-zA-Z0-9])?$").unwrap()
    });

    if namespace.is_empty() {
        Err(ValidationError::NamespaceEmpty)
    } else if namespace.len() > MAX_FIELD_LENGTH {
        Err(ValidationError::NamespaceTooLong)
    } else if namespace.ends_with(".atom") || namespace.ends_with(".git") {
        Err(ValidationError::NamespaceInvalidSuffix)
    } else if !RE_VALID_NAMESPACE.is_match(namespace) {
        Err(ValidationError::NamespaceInvalid)
    } else {
        Ok(())
    }
}

pub fn validate_project(project: &str) -> Result<(), ValidationError> {
    static RE_VALID_PROJECT: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"^[a-zA-Z0-9](?:[a-zA-Z0-9_.\-]*[a-zA-Z0-9])?$").unwrap()
    });

    if project.is_empty() {
        Err(ValidationError::ProjectEmpty)
    } else if project.len() > MAX_FIELD_LENGTH {
        Err(ValidationError::ProjectTooLong)
    } else if project.ends_with(".atom") || project.ends_with(".git") {
        Err(ValidationError::ProjectInvalidSuffix)
    } else if !RE_VALID_PROJECT.is_match(project) {
        Err(ValidationError::ProjectInvalid)
    } else {
        Ok(())
    }
}

pub fn validate_workflow_filepath(filepath: &str) -> Result<(), ValidationError> {
    if filepath.is_empty() {
        Err(ValidationError::WorkflowFilepathEmpty)
    } else if filepath.len() > MAX_FIELD_LENGTH {
        Err(ValidationError::WorkflowFilepathTooLong)
    } else if filepath.starts_with('/') {
        Err(ValidationError::WorkflowFilepathStartsWithSlash)
    } else if filepath.ends_with('/') {
        Err(ValidationError::WorkflowFilepathEndsWithSlash)
    } else if !filepath.ends_with(".yml") && !filepath.ends_with(".yaml") {
        Err(ValidationError::WorkflowFilepathMissingSuffix)
    } else {
        Ok(())
    }
}

pub fn validate_environment(env: &str) -> Result<(), ValidationError> {
    // see https://docs.gitlab.com/ci/yaml/#environment

    static RE_VALID_ENVIRONMENT: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^[a-zA-Z0-9 \-_/${}]+$").unwrap());

    if env.is_empty() {
        Err(ValidationError::EnvironmentEmptyString)
    } else if env.len() > MAX_FIELD_LENGTH {
        Err(ValidationError::EnvironmentTooLong)
    } else if !RE_VALID_ENVIRONMENT.is_match(env) {
        Err(ValidationError::EnvironmentInvalidChars)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use insta::assert_snapshot;

    #[test]
    fn test_validate_namespace() {
        assert_snapshot!(assert_err!(validate_namespace("")), @"GitLab namespace may not be empty");
        assert_snapshot!(assert_err!(validate_namespace(&"x".repeat(256))), @"GitLab namespace is too long (maximum is 255 characters)");
        assert_snapshot!(assert_err!(validate_namespace("-")), @"Invalid GitLab namespace");
        assert_snapshot!(assert_err!(validate_namespace("_")), @"Invalid GitLab namespace");
        assert_snapshot!(assert_err!(validate_namespace("-start")), @"Invalid GitLab namespace");
        assert_snapshot!(assert_err!(validate_namespace("end-")), @"Invalid GitLab namespace");
        assert_snapshot!(assert_err!(validate_namespace("invalid@chars")), @"Invalid GitLab namespace");
        assert_snapshot!(assert_err!(validate_namespace("foo+bar")), @"Invalid GitLab namespace");
        assert_snapshot!(assert_err!(validate_namespace("foo.atom")), @"GitLab namespace cannot end with .atom or .git");
        assert_snapshot!(assert_err!(validate_namespace("foo.git")), @"GitLab namespace cannot end with .atom or .git");

        assert_ok!(validate_namespace("a"));
        assert_ok!(validate_namespace("foo"));
        assert_ok!(validate_namespace("foo-bar"));
        assert_ok!(validate_namespace("foo_bar"));
        assert_ok!(validate_namespace("foo.bar"));
        assert_ok!(validate_namespace("foo/bar"));
        assert_ok!(validate_namespace("foo/bar/baz"));
    }

    #[test]
    fn test_validate_project() {
        assert_snapshot!(assert_err!(validate_project("")), @"GitLab project name may not be empty");
        assert_snapshot!(assert_err!(validate_project(&"x".repeat(256))), @"GitLab project name is too long (maximum is 255 characters)");
        assert_snapshot!(assert_err!(validate_project("-")), @"Invalid GitLab project name");
        assert_snapshot!(assert_err!(validate_project("_")), @"Invalid GitLab project name");
        assert_snapshot!(assert_err!(validate_project("-start")), @"Invalid GitLab project name");
        assert_snapshot!(assert_err!(validate_project("end-")), @"Invalid GitLab project name");
        assert_snapshot!(assert_err!(validate_project("invalid/chars")), @"Invalid GitLab project name");
        assert_snapshot!(assert_err!(validate_project("foo.atom")), @"GitLab project name cannot end with .atom or .git");
        assert_snapshot!(assert_err!(validate_project("foo.git")), @"GitLab project name cannot end with .atom or .git");

        assert_ok!(validate_project("a"));
        assert_ok!(validate_project("foo"));
        assert_ok!(validate_project("foo-bar"));
        assert_ok!(validate_project("foo_bar"));
        assert_ok!(validate_project("foo.bar"));
    }

    #[test]
    fn test_validate_workflow_filepath() {
        assert_snapshot!(assert_err!(validate_workflow_filepath("")), @"Workflow filepath may not be empty");
        assert_snapshot!(assert_err!(validate_workflow_filepath(&"x".repeat(256))), @"Workflow filepath is too long (maximum is 255 characters)");
        assert_snapshot!(assert_err!(validate_workflow_filepath("/starts-with-slash.yml")), @"Workflow filepath cannot start with /");
        assert_snapshot!(assert_err!(validate_workflow_filepath("ends-with-slash/")), @"Workflow filepath cannot end with /");
        assert_snapshot!(assert_err!(validate_workflow_filepath("no-suffix")), @"Workflow filepath must end with `.yml` or `.yaml`");

        assert_ok!(validate_workflow_filepath(".gitlab-ci.yml"));
        assert_ok!(validate_workflow_filepath(".gitlab-ci.yaml"));
        assert_ok!(validate_workflow_filepath("publish.yml"));
        assert_ok!(validate_workflow_filepath(".gitlab/ci/publish.yml"));
        assert_ok!(validate_workflow_filepath("ci/publish.yaml"));
    }

    #[test]
    fn test_validate_environment() {
        assert_snapshot!(assert_err!(validate_environment("")), @"Environment name may not be empty (use `null` to omit)");
        assert_snapshot!(assert_err!(validate_environment(&"x".repeat(256))), @"Environment name is too long (maximum is 255 characters)");
        assert_snapshot!(assert_err!(validate_environment("invalid@chars")), @"Environment name contains invalid characters");
        assert_snapshot!(assert_err!(validate_environment("invalid.dot")), @"Environment name contains invalid characters");

        assert_ok!(validate_environment("production"));
        assert_ok!(validate_environment("staging"));
        assert_ok!(validate_environment("prod-us-east"));
        assert_ok!(validate_environment("env_name"));
        assert_ok!(validate_environment("path/to/env"));
        assert_ok!(validate_environment("with space"));
    }
}
