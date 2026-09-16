#![doc = include_str!("../README.md")]

use anyhow::Context;
use bon::Builder;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde::{Serialize, Serializer};
use std::time::Duration;

const DEFAULT_SITE: &str = "datadoghq.com";

/// Per-request timeout for a single submission.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// A client for submitting data to the Datadog API.
#[derive(Builder, Debug)]
pub struct DatadogClient {
    http_client: Client,
    api_key: SecretString,
    #[builder(default = format!("https://api.{DEFAULT_SITE}"))]
    base_url: String,
}

impl<S: datadog_client_builder::State> DatadogClientBuilder<S> {
    /// Uses the API endpoint for the given Datadog site.
    pub fn site(
        self,
        site: impl AsRef<str>,
    ) -> DatadogClientBuilder<datadog_client_builder::SetBaseUrl<S>>
    where
        S::BaseUrl: datadog_client_builder::IsUnset,
    {
        self.base_url(format!("https://api.{}", site.as_ref()))
    }
}

impl DatadogClient {
    /// Submits a batch of service checks to Datadog.
    pub async fn submit_service_checks(&self, checks: &[ServiceCheck]) -> anyhow::Result<()> {
        self.submit("/api/v1/check_run", checks).await
    }

    async fn submit<T>(&self, path: &str, body: &T) -> anyhow::Result<()>
    where
        T: Serialize + ?Sized,
    {
        let url = self.api_url(path);

        let response = self
            .http_client
            .post(url)
            .header("DD-API-KEY", self.api_key.expose_secret())
            .timeout(REQUEST_TIMEOUT)
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Datadog")?;

        response
            .error_for_status()
            .context("Datadog returned an error response")?;

        Ok(())
    }

    /// Builds a URL for a Datadog API endpoint.
    fn api_url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }
}

/// A service check submitted to Datadog.
#[derive(Builder, Debug, PartialEq, Serialize)]
pub struct ServiceCheck {
    #[builder(into)]
    check: String,
    #[builder(into)]
    host_name: String,
    status: ServiceCheckStatus,
    #[builder(into)]
    message: String,
    tags: Vec<String>,
}

/// The status of a Datadog service check.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ServiceCheckStatus {
    /// The service is operating normally.
    Ok,
    /// The service may require attention.
    Warning,
    /// The service is not operating normally.
    Critical,
    /// The service status could not be determined.
    Unknown,
}

impl Serialize for ServiceCheckStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = match self {
            Self::Ok => 0,
            Self::Warning => 1,
            Self::Critical => 2,
            Self::Unknown => 3,
        };

        serializer.serialize_u8(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok, assert_some, assert_some_eq};
    use insta::assert_snapshot;
    use mockito::{Matcher, Server, ServerOpts};

    const TEST_API_KEY: &str = "test-api-key";

    async fn mock_server() -> Server {
        Server::new_with_opts_async(ServerOpts {
            assert_on_drop: true,
            ..Default::default()
        })
        .await
    }

    fn client_with_server(server: &Server) -> DatadogClient {
        DatadogClient::builder()
            .http_client(Client::new())
            .api_key(TEST_API_KEY.to_string().into())
            .base_url(server.url())
            .build()
    }

    fn service_check(check: &str, status: ServiceCheckStatus, message: &str) -> ServiceCheck {
        ServiceCheck::builder()
            .check(check)
            .host_name("crates.io")
            .status(status)
            .message(message)
            .tags(vec![
                "env:test".to_string(),
                "service:crates_io".to_string(),
            ])
            .build()
    }

    #[test]
    fn uses_site_for_default_base_url() {
        let client = DatadogClient::builder()
            .http_client(Client::new())
            .api_key(TEST_API_KEY.to_string().into())
            .site("datadoghq.eu")
            .build();

        assert_eq!(
            client.api_url("/api/v1/check_run"),
            "https://api.datadoghq.eu/api/v1/check_run"
        );
    }

    #[test]
    fn uses_explicit_base_url() {
        let client = DatadogClient::builder()
            .http_client(Client::new())
            .api_key(TEST_API_KEY.to_string().into())
            .base_url("http://localhost".to_string())
            .build();

        assert_eq!(
            client.api_url("/api/v1/check_run"),
            "http://localhost/api/v1/check_run"
        );
    }

    #[tokio::test]
    async fn submits_service_checks() {
        let mut server = mock_server().await;
        let _mock = server
            .mock("POST", "/api/v1/check_run")
            .match_header("dd-api-key", TEST_API_KEY)
            .match_body(Matcher::JsonString(
                r#"[
                    {
                        "check": "crates_io.background_jobs.healthy",
                        "host_name": "crates.io",
                        "status": 0,
                        "message": "No stalled background jobs",
                        "tags": ["env:test", "service:crates_io"]
                    },
                    {
                        "check": "crates_io.spam_attack.detected",
                        "host_name": "crates.io",
                        "status": 2,
                        "message": "Spam crate published",
                        "tags": ["env:test", "service:crates_io"]
                    }
                ]"#
                .to_string(),
            ))
            .with_status(202)
            .expect(1)
            .create_async()
            .await;

        let checks = [
            service_check(
                "crates_io.background_jobs.healthy",
                ServiceCheckStatus::Ok,
                "No stalled background jobs",
            ),
            service_check(
                "crates_io.spam_attack.detected",
                ServiceCheckStatus::Critical,
                "Spam crate published",
            ),
        ];
        let client = client_with_server(&server);
        assert_ok!(client.submit_service_checks(&checks).await);
    }

    #[tokio::test]
    async fn reports_rejected_service_check_submissions() {
        let mut server = mock_server().await;
        let _mock = server
            .mock("POST", "/api/v1/check_run")
            .with_status(403)
            .expect(1)
            .create_async()
            .await;

        let check = service_check(
            "crates_io.background_jobs.healthy",
            ServiceCheckStatus::Critical,
            "Stalled background jobs",
        );
        let client = client_with_server(&server);
        let error = assert_err!(client.submit_service_checks(&[check]).await);
        assert_snapshot!(error, @"Datadog returned an error response");

        let http_error = assert_some!(error.downcast_ref::<reqwest::Error>());
        assert_some_eq!(http_error.status(), reqwest::StatusCode::FORBIDDEN);
    }
}
