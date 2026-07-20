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
    /// Submits a batch of metric series to Datadog.
    pub async fn submit_metrics(&self, series: &[Series]) -> anyhow::Result<()> {
        let url = self.api_url("/api/v2/series");
        let body = MetricsBody { series };

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

#[derive(Serialize)]
struct MetricsBody<'a> {
    series: &'a [Series],
}

/// A metric series submitted to Datadog.
#[derive(Builder, Debug, PartialEq, Serialize)]
pub struct Series {
    #[builder(into)]
    metric: String,
    #[serde(rename = "type")]
    kind: MetricType,
    points: Vec<Point>,
    resources: Vec<Resource>,
    tags: Vec<String>,
}

/// The type of a Datadog metric series.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MetricType {
    /// A count representing the total number of event occurrences in one time
    /// interval.
    Count,
    /// A rate representing the normalized number of event occurrences per
    /// second.
    Rate,
    /// A gauge representing a value at a specific point in time.
    Gauge,
}

impl Serialize for MetricType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = match self {
            Self::Count => 1,
            Self::Rate => 2,
            Self::Gauge => 3,
        };

        serializer.serialize_u8(value)
    }
}

/// A timestamped value in a Datadog metric series.
#[derive(Builder, Debug, PartialEq, Serialize)]
pub struct Point {
    timestamp: i64,
    value: f64,
}

/// A resource associated with a Datadog metric series.
#[derive(Builder, Clone, Debug, PartialEq, Serialize)]
pub struct Resource {
    #[builder(into)]
    #[serde(rename = "type")]
    kind: String,
    #[builder(into)]
    name: String,
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

    fn metric_series() -> Series {
        let point = Point::builder()
            .timestamp(1_753_000_000)
            .value(42.0)
            .build();

        let resource = Resource::builder().kind("host").name("crates.io").build();

        Series::builder()
            .metric("crates_io.background_jobs")
            .kind(MetricType::Gauge)
            .points(vec![point])
            .resources(vec![resource])
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
            client.api_url("/api/v2/series"),
            "https://api.datadoghq.eu/api/v2/series"
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
            client.api_url("/api/v2/series"),
            "http://localhost/api/v2/series"
        );
    }

    #[tokio::test]
    async fn submits_metric_series() {
        let mut server = mock_server().await;
        let _mock = server
            .mock("POST", "/api/v2/series")
            .match_header("dd-api-key", TEST_API_KEY)
            .match_body(Matcher::JsonString(
                r#"{
                    "series": [{
                        "metric": "crates_io.background_jobs",
                        "type": 3,
                        "points": [{
                            "timestamp": 1753000000,
                            "value": 42.0
                        }],
                        "resources": [{
                            "type": "host",
                            "name": "crates.io"
                        }],
                        "tags": ["env:test", "service:crates_io"]
                    }]
                }"#
                .to_string(),
            ))
            .with_status(202)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        assert_ok!(client.submit_metrics(&[metric_series()]).await);
    }

    #[tokio::test]
    async fn reports_rejected_metric_submissions() {
        let mut server = mock_server().await;
        let _mock = server
            .mock("POST", "/api/v2/series")
            .with_status(403)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        let error = assert_err!(client.submit_metrics(&[metric_series()]).await);
        assert_snapshot!(error, @"Datadog returned an error response");

        let http_error = assert_some!(error.downcast_ref::<reqwest::Error>());
        assert_some_eq!(http_error.status(), reqwest::StatusCode::FORBIDDEN);
    }
}
