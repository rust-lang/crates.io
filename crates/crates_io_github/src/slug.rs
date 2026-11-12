use url::Url;

/// Parses `(owner, repo)` from a GitHub repository URL.
///
/// Accepts `ssh://git@github.com/OWNER/REPO.git` and
/// `https://github.com/OWNER/REPO` (with or without a trailing `.git`).
/// Rejects URLs that do not point at `github.com` or that do not contain
/// both an owner and a repository name.
pub fn parse_github_slug(url: &Url) -> Result<(String, String), ParseSlugError> {
    match url.host_str() {
        Some("github.com") => {}
        Some(host) => return Err(ParseSlugError::UnsupportedHost(host.to_string())),
        None => return Err(ParseSlugError::MissingHost),
    }

    let mut segments = url
        .path_segments()
        .ok_or(ParseSlugError::MissingPath)?
        .filter(|segment| !segment.is_empty());

    let owner = segments.next().ok_or(ParseSlugError::MissingOwner)?;
    let repo = segments.next().ok_or(ParseSlugError::MissingRepo)?;

    let repo = repo.trim_suffix(".git");
    if repo.is_empty() {
        return Err(ParseSlugError::MissingRepo);
    }

    Ok((owner.to_string(), repo.to_string()))
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseSlugError {
    #[error("URL is missing a host")]
    MissingHost,
    #[error("unsupported host `{0}`, only `github.com` URLs are accepted")]
    UnsupportedHost(String),
    #[error("URL has no path")]
    MissingPath,
    #[error("URL is missing the repository owner")]
    MissingOwner,
    #[error("URL is missing the repository name")]
    MissingRepo,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Result<(String, String), ParseSlugError> {
        let url = Url::parse(input).expect("valid URL in test input");
        parse_github_slug(&url)
    }

    #[test]
    fn parses_ssh_url_with_dot_git() {
        let (owner, repo) = parse("ssh://git@github.com/rust-lang/crates.io-index.git").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "crates.io-index");
    }

    #[test]
    fn parses_https_url_without_dot_git() {
        let (owner, repo) = parse("https://github.com/rust-lang/crates.io-index").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "crates.io-index");
    }

    #[test]
    fn parses_https_url_with_dot_git() {
        let (owner, repo) = parse("https://github.com/rust-lang/crates.io-index.git").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "crates.io-index");
    }

    #[test]
    fn ignores_trailing_slash() {
        let (owner, repo) = parse("https://github.com/rust-lang/crates.io-index/").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "crates.io-index");
    }

    #[test]
    fn parses_https_url_with_extra_path_segments() {
        let (owner, repo) = parse("https://github.com/rust-lang/crates.io/pull/123").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "crates.io");
    }

    #[test]
    fn rejects_non_github_host() {
        let err = parse("https://gitlab.com/rust-lang/crates.io-index").unwrap_err();
        assert_eq!(err, ParseSlugError::UnsupportedHost("gitlab.com".into()));
    }

    #[test]
    fn rejects_missing_host() {
        let err = parse("ssh:///rust-lang/crates.io-index.git").unwrap_err();
        assert_eq!(err, ParseSlugError::MissingHost);
    }

    #[test]
    fn rejects_missing_repo() {
        let err = parse("https://github.com/rust-lang").unwrap_err();
        assert_eq!(err, ParseSlugError::MissingRepo);
    }

    #[test]
    fn rejects_missing_owner_and_repo() {
        let err = parse("https://github.com/").unwrap_err();
        assert_eq!(err, ParseSlugError::MissingOwner);
    }

    #[test]
    fn rejects_bare_dot_git_repo_name() {
        let err = parse("https://github.com/rust-lang/.git").unwrap_err();
        assert_eq!(err, ParseSlugError::MissingRepo);
    }
}
