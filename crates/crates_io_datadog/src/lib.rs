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
    #[builder(default = DEFAULT_SITE.to_string())]
    site: String,
}

impl DatadogClient {
    /// Submits a batch of metric series to Datadog.
    pub async fn submit_metrics(&self, series: &[Series]) -> anyhow::Result<()> {
        let url = format!("https://api.{}/api/v2/series", self.site);
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
