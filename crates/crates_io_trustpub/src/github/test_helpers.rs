use crate::github::GITHUB_ISSUER_URL;
use crate::test_keys::encode_for_testing;
use bon::bon;
use serde_json::json;

pub const AUDIENCE: &str = "crates.io";

/// A struct representing all the claims in a GitHub Actions OIDC token.
///
/// This struct is used to create a JWT for testing purposes.
#[derive(Debug, serde::Serialize)]
pub struct FullGitHubClaims {
    pub iss: String,
    pub nbf: i64,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
    pub sub: String,
    pub aud: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(rename = "ref")]
    pub r#ref: String,
    pub sha: String,
    pub repository: String,
    pub repository_owner: String,
    pub actor_id: String,
    pub repository_visibility: String,
    pub repository_id: String,
    pub repository_owner_id: String,
    pub run_id: String,
    pub run_number: String,
    pub run_attempt: String,
    pub runner_environment: String,
    pub actor: String,
    pub workflow: String,
    pub head_ref: String,
    pub base_ref: String,
    pub event_name: String,
    pub ref_type: String,
    pub workflow_ref: String,
}

#[bon]
impl FullGitHubClaims {
    #[builder]
    pub fn new(
        owner_id: i32,
        owner_name: &str,
        repository_name: &str,
        workflow_filename: &str,
        environment: Option<&str>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();

        Self {
            iss: GITHUB_ISSUER_URL.into(),
            nbf: now,
            iat: now,
            exp: now + 30 * 60,
            jti: "example-id".into(),
            sub: format!("repo:{owner_name}/{repository_name}"),
            aud: AUDIENCE.into(),

            environment: environment.map(|s| s.into()),
            r#ref: "refs/heads/main".into(),
            sha: "example-sha".into(),
            repository: format!("{owner_name}/{repository_name}"),
            repository_owner: owner_name.into(),
            actor_id: "12".into(),
            repository_visibility: "private".into(),
            repository_id: "74".into(),
            repository_owner_id: owner_id.to_string(),
            run_id: "example-run-id".into(),
            run_number: "10".into(),
            run_attempt: "2".into(),
            runner_environment: "github-hosted".into(),
            actor: "octocat".into(),
            workflow: "example-workflow".into(),
            head_ref: "".into(),
            base_ref: "".into(),
            event_name: "workflow_dispatch".into(),
            ref_type: "branch".into(),
            workflow_ref: format!(
                "{owner_name}/{repository_name}/.github/workflows/{workflow_filename}@refs/heads/main"
            ),
        }
    }

    pub fn encoded(&self) -> anyhow::Result<String> {
        Ok(encode_for_testing(self)?)
    }

    pub fn as_exchange_body(&self) -> anyhow::Result<String> {
        let jwt = self.encoded()?;
        Ok(serde_json::to_string(&json!({ "jwt": jwt }))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ok;
    use insta::assert_json_snapshot;

    #[test]
    fn test_github_claims() {
        let claims = FullGitHubClaims::builder()
            .owner_id(123)
            .owner_name("octocat")
            .repository_name("hello-world")
            .workflow_filename("ci.yml")
            .build();

        assert_json_snapshot!(claims, {
            ".nbf" => "[timestamp]",
            ".iat" => "[timestamp]",
            ".exp" => "[timestamp]",
        });

        let encoded = assert_ok!(claims.encoded());
        assert!(!encoded.is_empty());

        let exchange_body = assert_ok!(claims.as_exchange_body());
        assert!(exchange_body.contains(&encoded));
    }
}
