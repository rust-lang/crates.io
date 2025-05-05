use std::sync::LazyLock;

const MAX_FIELD_LENGTH: usize = 255;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("GitHub repository owner name may not be empty")]
    OwnerEmpty,
    #[error("GitHub repository owner name is too long (maximum is {MAX_FIELD_LENGTH} characters)")]
    OwnerTooLong,
    #[error("Invalid GitHub repository owner name")]
    OwnerInvalid,

    #[error("GitHub repository name may not be empty")]
    RepoEmpty,
    #[error("GitHub repository name is too long (maximum is {MAX_FIELD_LENGTH} characters)")]
    RepoTooLong,
    #[error("Invalid GitHub repository name")]
    RepoInvalid,

    #[error("Workflow filename may not be empty")]
    WorkflowFilenameEmpty,
    #[error("Workflow filename is too long (maximum is {MAX_FIELD_LENGTH} characters)")]
    WorkflowFilenameTooLong,
    #[error("Workflow filename must end with `.yml` or `.yaml`")]
    WorkflowFilenameMissingSuffix,
    #[error("Workflow filename must be a filename only, without directories")]
    WorkflowFilenameContainsSlash,

    #[error("Environment name may not be empty (use `null` to omit)")]
    EnvironmentEmptyString,
    #[error("Environment name is too long (maximum is {MAX_FIELD_LENGTH} characters)")]
    EnvironmentTooLong,
    #[error("Environment name may not start with whitespace")]
    EnvironmentStartsWithWhitespace,
    #[error("Environment name may not end with whitespace")]
    EnvironmentEndsWithWhitespace,
    #[error(r#"Environment name must not contain non-printable characters or the characters "'", """, "`", ",", ";", "\""#)]
    EnvironmentInvalidChars,
}

pub fn validate_owner(owner: &str) -> Result<(), ValidationError> {
    static RE_VALID_GITHUB_OWNER: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9-]*$").unwrap());

    if owner.is_empty() {
        Err(ValidationError::OwnerEmpty)
    } else if owner.len() > MAX_FIELD_LENGTH {
        Err(ValidationError::OwnerTooLong)
    } else if !RE_VALID_GITHUB_OWNER.is_match(owner) {
        Err(ValidationError::OwnerInvalid)
    } else {
        Ok(())
    }
}

pub fn validate_repo(repo: &str) -> Result<(), ValidationError> {
    static RE_VALID_GITHUB_REPO: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^[a-zA-Z0-9-_.]+$").unwrap());

    if repo.is_empty() {
        Err(ValidationError::RepoEmpty)
    } else if repo.len() > MAX_FIELD_LENGTH {
        Err(ValidationError::RepoTooLong)
    } else if !RE_VALID_GITHUB_REPO.is_match(repo) {
        Err(ValidationError::RepoInvalid)
    } else {
        Ok(())
    }
}

pub fn validate_workflow_filename(filename: &str) -> Result<(), ValidationError> {
    if filename.is_empty() {
        Err(ValidationError::WorkflowFilenameEmpty)
    } else if filename.len() > MAX_FIELD_LENGTH {
        Err(ValidationError::WorkflowFilenameTooLong)
    } else if !filename.ends_with(".yml") && !filename.ends_with(".yaml") {
        Err(ValidationError::WorkflowFilenameMissingSuffix)
    } else if filename.contains('/') {
        Err(ValidationError::WorkflowFilenameContainsSlash)
    } else {
        Ok(())
    }
}

pub fn validate_environment(env: &str) -> Result<(), ValidationError> {
    static RE_INVALID_ENVIRONMENT_CHARS: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#"[\x00-\x1F\x7F'"`,;\\]"#).unwrap());

    if env.is_empty() {
        Err(ValidationError::EnvironmentEmptyString)
    } else if env.len() > MAX_FIELD_LENGTH {
        Err(ValidationError::EnvironmentTooLong)
    } else if env.starts_with(" ") {
        Err(ValidationError::EnvironmentStartsWithWhitespace)
    } else if env.ends_with(" ") {
        Err(ValidationError::EnvironmentEndsWithWhitespace)
    } else if RE_INVALID_ENVIRONMENT_CHARS.is_match(env) {
        Err(ValidationError::EnvironmentInvalidChars)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_err;
    use insta::assert_snapshot;

    #[test]
    fn test_validate_owner() {
        assert_snapshot!(assert_err!(validate_owner("")), @"GitHub repository owner name may not be empty");
        assert_snapshot!(assert_err!(validate_owner(&"x".repeat(256))), @"GitHub repository owner name is too long (maximum is 255 characters)");
        assert_snapshot!(assert_err!(validate_owner("invalid_characters@")), @"Invalid GitHub repository owner name");
    }

    #[test]
    fn test_validate_repo() {
        assert_snapshot!(assert_err!(validate_repo("")), @"GitHub repository name may not be empty");
        assert_snapshot!(assert_err!(validate_repo(&"x".repeat(256))), @"GitHub repository name is too long (maximum is 255 characters)");
        assert_snapshot!(assert_err!(validate_repo("$invalid#characters")), @"Invalid GitHub repository name");
    }

    #[test]
    fn test_validate_workflow_filename() {
        assert_snapshot!(assert_err!(validate_workflow_filename("")), @"Workflow filename may not be empty");
        assert_snapshot!(assert_err!(validate_workflow_filename(&"x".repeat(256))), @"Workflow filename is too long (maximum is 255 characters)");
        assert_snapshot!(assert_err!(validate_workflow_filename("missing_suffix")), @"Workflow filename must end with `.yml` or `.yaml`");
        assert_snapshot!(assert_err!(validate_workflow_filename("/slash")), @"Workflow filename must end with `.yml` or `.yaml`");
        assert_snapshot!(assert_err!(validate_workflow_filename("/many/slashes")), @"Workflow filename must end with `.yml` or `.yaml`");
        assert_snapshot!(assert_err!(validate_workflow_filename("/slash.yml")), @"Workflow filename must be a filename only, without directories");
    }

    #[test]
    fn test_validate_environment() {
        assert_snapshot!(assert_err!(validate_environment("")), @"Environment name may not be empty (use `null` to omit)");
        assert_snapshot!(assert_err!(validate_environment(&"x".repeat(256))), @"Environment name is too long (maximum is 255 characters)");
        assert_snapshot!(assert_err!(validate_environment(" foo")), @"Environment name may not start with whitespace");
        assert_snapshot!(assert_err!(validate_environment("foo ")), @"Environment name may not end with whitespace");
        assert_snapshot!(assert_err!(validate_environment("'")), @r#"Environment name must not contain non-printable characters or the characters "'", """, "`", ",", ";", "\""#);
        assert_snapshot!(assert_err!(validate_environment("\"")), @r#"Environment name must not contain non-printable characters or the characters "'", """, "`", ",", ";", "\""#);
        assert_snapshot!(assert_err!(validate_environment("`")), @r#"Environment name must not contain non-printable characters or the characters "'", """, "`", ",", ";", "\""#);
        assert_snapshot!(assert_err!(validate_environment(",")), @r#"Environment name must not contain non-printable characters or the characters "'", """, "`", ",", ";", "\""#);
        assert_snapshot!(assert_err!(validate_environment(";")), @r#"Environment name must not contain non-printable characters or the characters "'", """, "`", ",", ";", "\""#);
        assert_snapshot!(assert_err!(validate_environment("\\")), @r#"Environment name must not contain non-printable characters or the characters "'", """, "`", ",", ";", "\""#);
        assert_snapshot!(assert_err!(validate_environment("\x00")), @r#"Environment name must not contain non-printable characters or the characters "'", """, "`", ",", ";", "\""#);
        assert_snapshot!(assert_err!(validate_environment("\x1f")), @r#"Environment name must not contain non-printable characters or the characters "'", """, "`", ",", ";", "\""#);
        assert_snapshot!(assert_err!(validate_environment("\x7f")), @r#"Environment name must not contain non-printable characters or the characters "'", """, "`", ",", ";", "\""#);
        assert_snapshot!(assert_err!(validate_environment("\t")), @r#"Environment name must not contain non-printable characters or the characters "'", """, "`", ",", ";", "\""#);
        assert_snapshot!(assert_err!(validate_environment("\r")), @r#"Environment name must not contain non-printable characters or the characters "'", """, "`", ",", ";", "\""#);
        assert_snapshot!(assert_err!(validate_environment("\n")), @r#"Environment name must not contain non-printable characters or the characters "'", """, "`", ",", ";", "\""#);
    }
}
