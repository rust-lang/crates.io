mod claims;
#[cfg(any(test, feature = "test-helpers"))]
pub mod test_helpers;
mod workflows;

pub use self::claims::GitLabClaims;

pub const GITLAB_ISSUER_URL: &str = "https://gitlab.com";
