mod claims;
#[cfg(any(test, feature = "test-helpers"))]
pub mod test_helpers;
pub mod validation;
mod workflows;

pub use claims::GitHubClaims;

pub const GITHUB_ISSUER_URL: &str = "https://token.actions.githubusercontent.com";
