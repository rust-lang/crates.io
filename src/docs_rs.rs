use async_trait::async_trait;
use http::StatusCode;
use mockall::automock;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum DocsRsError {
    /// The rebuild couldn't be triggered.
    /// The reason is passed in the given error message.
    #[error("Bad request: {0}")]
    BadRequest(String),
    /// The request was rate limited by the server.
    /// This is the NGINX level rate limit for requests coming from a single IP.
    /// This is _not_ the rate limit that docs.rs might apply for rebuilds of the same crate
    /// (AKA: "rebuild too often").
    #[error("rate limited")]
    RateLimited,
    #[error("unauthorized")]
    Unauthorized,
    /// crate or version not found on docs.rs.
    /// This can be temporary directly after a release until the first
    /// docs build was started for the crate.
    #[error("crate or version not found on docs.rs")]
    NotFound,
    #[error(transparent)]
    Other(anyhow::Error),
}

#[automock]
#[async_trait]
pub trait DocsRsClient: Send + Sync {
    async fn rebuild_docs(&self, name: &str, version: &str) -> Result<(), DocsRsError>;
}

pub(crate) struct RealDocsRsClient {
    client: reqwest::Client,
    base_url: Url,
    api_token: String,
}

impl RealDocsRsClient {
    pub fn new(base_url: impl Into<Url>, api_token: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            api_token: api_token.into(),
        }
    }
}

#[async_trait]
impl DocsRsClient for RealDocsRsClient {
    async fn rebuild_docs(&self, name: &str, version: &str) -> Result<(), DocsRsError> {
        let target_url = self
            .base_url
            .join(&format!("/crate/{name}/{version}/rebuild"))
            .map_err(|err| DocsRsError::Other(err.into()))?;

        let response = self
            .client
            .post(target_url)
            .bearer_auth(&self.api_token)
            .send()
            .await
            .map_err(|err| DocsRsError::Other(err.into()))?;

        match response.status() {
            StatusCode::CREATED => Ok(()),
            StatusCode::NOT_FOUND => Err(DocsRsError::NotFound),
            StatusCode::TOO_MANY_REQUESTS => Err(DocsRsError::RateLimited),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(DocsRsError::Unauthorized),
            StatusCode::BAD_REQUEST => {
                #[derive(Deserialize)]
                struct BadRequestResponse {
                    message: String,
                }

                let error_response: BadRequestResponse = response
                    .json()
                    .await
                    .map_err(|err| DocsRsError::Other(err.into()))?;

                Err(DocsRsError::BadRequest(error_response.message))
            }
            _ => Err(DocsRsError::Other(anyhow::anyhow!(
                "Unexpected response from docs.rs: {}\n{}",
                response.status(),
                response.text().await.unwrap_or_default()
            ))),
        }
    }
}

/// Builds an [DocsRsClient] implementation based on the [crate::config::Server]
pub fn docs_rs_client(config: &crate::config::Server) -> Box<dyn DocsRsClient + Send + Sync> {
    if let Some(api_token) = &config.docs_rs_api_token {
        Box::new(RealDocsRsClient::new(config.docs_rs_url.clone(), api_token))
    } else {
        Box::new(MockDocsRsClient::new())
    }
}
