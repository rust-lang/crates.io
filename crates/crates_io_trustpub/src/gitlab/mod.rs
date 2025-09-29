mod claims;
mod workflows;

pub use self::claims::GitLabClaims;

pub const GITLAB_ISSUER_URL: &str = "https://gitlab.com";
