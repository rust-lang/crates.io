#![doc = include_str!("../README.md")]

use async_trait::async_trait;
use crates_io_env_vars::{var, var_parsed};
use http::StatusCode;
use serde::Deserialize;
use tracing::warn;
use url::Url;

pub const DEFAULT_BASE_URL: &str = "https://docs.rs";

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

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait DocsRsClient: Send + Sync {
    async fn rebuild_docs(&self, name: &str, version: &str) -> Result<(), DocsRsError>;
}

pub struct RealDocsRsClient {
    client: reqwest::Client,
    base_url: Url,
    api_token: String,
}

impl RealDocsRsClient {
    pub fn new(base_url: Url, api_token: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("crates.io")
                .build()
                .unwrap(),
            base_url,
            api_token: api_token.into(),
        }
    }

    pub fn from_environment() -> Option<Self> {
        let base_url: Url = match var_parsed("DOCS_RS_BASE_URL") {
            Ok(Some(url)) => url,
            Ok(None) => Url::parse(DEFAULT_BASE_URL).unwrap(),
            Err(err) => {
                warn!(?err, "Failed to parse DOCS_RS_BASE_URL");
                return None;
            }
        };

        let api_token = var("DOCS_RS_API_TOKEN").ok()??;

        Some(Self::new(base_url, api_token))
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

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_matches;
    use test_case::test_case;

    async fn mock(
        krate: &str,
        version: &str,
        status: StatusCode,
    ) -> (mockito::ServerGuard, mockito::Mock) {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", &format!("/crate/{krate}/{version}/rebuild")[..])
            .match_header("Authorization", "Bearer test_token")
            .with_status(StatusCode::CREATED.as_u16().into())
            .with_status(status.as_u16().into());

        (server, mock)
    }

    #[tokio::test]
    async fn test_ok() -> anyhow::Result<()> {
        let (server, mock) = mock("krate", "0.1.0", StatusCode::CREATED).await;
        mock.create();

        let docs_rs = RealDocsRsClient::new(Url::parse(&server.url())?, "test_token");

        docs_rs.rebuild_docs("krate", "0.1.0").await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_crate_not_found() -> anyhow::Result<()> {
        let (server, mock) = mock("krate", "0.1.0", StatusCode::NOT_FOUND).await;
        mock.create();

        let docs_rs = RealDocsRsClient::new(Url::parse(&server.url())?, "test_token");

        assert_matches!(
            docs_rs.rebuild_docs("krate", "0.1.0").await,
            Err(DocsRsError::NotFound)
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_crate_too_many_requests() -> anyhow::Result<()> {
        let (server, mock) = mock("krate", "0.1.0", StatusCode::TOO_MANY_REQUESTS).await;
        mock.create();

        let docs_rs = RealDocsRsClient::new(Url::parse(&server.url())?, "test_token");

        assert_matches!(
            docs_rs.rebuild_docs("krate", "0.1.0").await,
            Err(DocsRsError::RateLimited)
        );

        Ok(())
    }

    #[tokio::test]
    #[test_case(StatusCode::UNAUTHORIZED)]
    #[test_case(StatusCode::FORBIDDEN)]
    async fn test_permissions(status: StatusCode) -> anyhow::Result<()> {
        let (server, mock) = mock("krate", "0.1.0", status).await;
        mock.create();

        let docs_rs = RealDocsRsClient::new(Url::parse(&server.url())?, "test_token");

        assert_matches!(
            docs_rs.rebuild_docs("krate", "0.1.0").await,
            Err(DocsRsError::Unauthorized)
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_bad_request_message() -> anyhow::Result<()> {
        let (server, mock) = mock("krate", "0.1.0", StatusCode::BAD_REQUEST).await;
        let body = serde_json::to_vec(&serde_json::json!({
            "message": "some error message"
        }))?;
        mock.with_body(&body).create();

        let docs_rs = RealDocsRsClient::new(Url::parse(&server.url())?, "test_token");

        assert_matches!(
            docs_rs.rebuild_docs("krate", "0.1.0").await,
            Err(DocsRsError::BadRequest(msg)) if msg == "some error message"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_server_error() -> anyhow::Result<()> {
        let (server, mock) = mock("krate", "0.1.0", StatusCode::INTERNAL_SERVER_ERROR).await;
        mock.create();

        let docs_rs = RealDocsRsClient::new(Url::parse(&server.url())?, "test_token");

        assert_matches!(
            docs_rs.rebuild_docs("krate", "0.1.0").await,
            Err(DocsRsError::Other(_))
        );

        Ok(())
    }
}
