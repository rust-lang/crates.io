use super::GITLAB_ISSUER_URL;
use crate::test_keys::encode_for_testing;
use bon::bon;
use serde_json::json;

pub const AUDIENCE: &str = "crates.io";

/// A struct representing all the claims in a GitLab CI OIDC token.
///
/// This struct is used to create a JWT for testing purposes.
#[derive(Debug, serde::Serialize)]
pub struct FullGitLabClaims {
    pub iss: String,
    pub nbf: i64,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
    pub sub: String,
    pub aud: String,

    pub project_id: String,
    pub project_path: String,
    pub namespace_id: String,
    pub namespace_path: String,
    pub user_id: String,
    pub user_login: String,
    pub user_email: String,
    pub user_access_level: String,
    pub job_project_id: String,
    pub job_project_path: String,
    pub job_namespace_id: String,
    pub job_namespace_path: String,
    pub pipeline_id: String,
    pub pipeline_source: String,
    pub job_id: String,
    #[serde(rename = "ref")]
    pub r#ref: String,
    pub ref_type: String,
    pub ref_path: String,
    pub ref_protected: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_protected: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_action: Option<String>,
    pub runner_id: i64,
    pub runner_environment: String,
    pub sha: String,
    pub project_visibility: String,
    pub ci_config_ref_uri: String,
    pub ci_config_sha: String,
}

#[bon]
impl FullGitLabClaims {
    #[builder]
    pub fn new(
        namespace_id: &str,
        namespace: &str,
        project: &str,
        workflow_filepath: &str,
        environment: Option<&str>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();

        Self {
            iss: GITLAB_ISSUER_URL.into(),
            nbf: now,
            iat: now,
            exp: now + 60 * 60,
            jti: "example-id".into(),
            sub: format!("project_path:{namespace}/{project}:ref_type:branch:ref:main"),
            aud: AUDIENCE.into(),

            project_id: "74884433".into(),
            project_path: format!("{namespace}/{project}"),
            namespace_id: namespace_id.into(),
            namespace_path: namespace.into(),
            user_id: "39035".into(),
            user_login: namespace.into(),
            user_email: "foo@bar.cloud".into(),
            user_access_level: "owner".into(),
            job_project_id: "74884433".into(),
            job_project_path: format!("{namespace}/{project}"),
            job_namespace_id: namespace_id.into(),
            job_namespace_path: namespace.into(),
            pipeline_id: "2069090987".into(),
            pipeline_source: "push".into(),
            job_id: "11530106120".into(),
            r#ref: "main".into(),
            ref_type: "branch".into(),
            ref_path: "refs/heads/main".into(),
            ref_protected: "true".into(),
            environment: environment.map(|s| s.into()),
            environment_protected: environment.map(|_| "false".into()),
            deployment_tier: environment.map(|_| "other".into()),
            environment_action: environment.map(|_| "start".into()),
            runner_id: 12270840,
            runner_environment: "gitlab-hosted".into(),
            sha: "76719c2658b5c4423810d655a4624af1b38b7091".into(),
            project_visibility: "public".into(),
            ci_config_ref_uri: format!(
                "gitlab.com/{namespace}/{project}//{workflow_filepath}@refs/heads/main"
            ),
            ci_config_sha: "76719c2658b5c4423810d655a4624af1b38b7091".into(),
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
    fn test_gitlab_claims() {
        let claims = FullGitLabClaims::builder()
            .namespace_id("123")
            .namespace("octocat")
            .project("hello-world")
            .workflow_filepath(".gitlab-ci.yml")
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
