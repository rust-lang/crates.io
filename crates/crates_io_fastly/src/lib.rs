#![doc = include_str!("../README.md")]

use crates_io_version::user_agent;
use reqwest::header::{HeaderValue, InvalidHeaderValue};
use reqwest::{Client, ClientBuilder};
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;
use tracing::{debug, instrument, trace};

const API_BASE_URL: &str = "https://api.fastly.com";

#[derive(Debug, Error)]
pub enum Error {
    #[error("Wildcard invalidations are not supported for Fastly")]
    WildcardNotSupported,

    #[error("Invalid API token format")]
    InvalidApiToken(#[from] InvalidHeaderValue),

    #[error("Failed to `POST {url}`{}: {source}", status.map(|s| format!(" (status: {})", s)).unwrap_or_default())]
    PurgeFailed {
        url: String,
        status: Option<reqwest::StatusCode>,
        #[source]
        source: reqwest::Error,
    },
}

#[derive(Debug)]
pub struct Fastly {
    client: Client,
    api_token: SecretString,
    api_base_url: String,
}

impl Fastly {
    pub fn new(api_token: SecretString) -> Self {
        Self::with_api_base_url(api_token, API_BASE_URL.into())
    }

    /// Creates a Fastly client using the given API base URL.
    fn with_api_base_url(api_token: SecretString, api_base_url: String) -> Self {
        let client = ClientBuilder::new()
            .user_agent(user_agent())
            .build()
            .unwrap();

        Self {
            client,
            api_token,
            api_base_url,
        }
    }

    /// Invalidates a path on Fastly
    ///
    /// This method takes a path and invalidates the cached content on Fastly. The path must not
    /// contain a wildcard, since the Fastly API does not support wildcard invalidations. Paths are
    /// invalidated for both domains that are associated with the Fastly service.
    ///
    /// Requests are authenticated using a token that is sent in a header. The token is passed to
    /// the application as an environment variable.
    ///
    /// More information on Fastly's APIs for cache invalidations can be found here:
    /// <https://developer.fastly.com/reference/api/purging/>
    #[instrument(skip(self))]
    pub async fn purge_both_domains(&self, base_domain: &str, path: &str) -> Result<(), Error> {
        self.purge(base_domain, path).await?;

        let prefixed_domain = format!("fastly-{base_domain}");
        self.purge(&prefixed_domain, path).await?;

        Ok(())
    }

    /// Invalidates a path on Fastly
    ///
    /// This method takes a domain and path and invalidates the cached content
    /// on Fastly. The path must not contain a wildcard, since the Fastly API
    /// does not support wildcard invalidations.
    ///
    /// More information on Fastly's APIs for cache invalidations can be found here:
    /// <https://developer.fastly.com/reference/api/purging/>
    #[instrument(skip(self))]
    pub async fn purge(&self, domain: &str, path: &str) -> Result<(), Error> {
        if path.contains('*') {
            return Err(Error::WildcardNotSupported);
        }

        let path = path.trim_start_matches('/');
        let url = format!("{}/purge/{domain}/{path}", self.api_base_url);

        trace!(?url);

        debug!("sending invalidation request to Fastly");
        let response = self
            .client
            .post(&url)
            .header("Fastly-Key", self.token_header_value()?)
            .send()
            .await
            .map_err(|source| Error::PurgeFailed {
                url: url.clone(),
                status: None,
                source,
            })?;

        let status = response.status();

        match response.error_for_status_ref() {
            Ok(_) => {
                debug!(?status, "invalidation request accepted by Fastly");
                Ok(())
            }
            Err(error) => {
                let headers = response.headers().clone();
                let body = response.text().await;
                debug!(
                    ?status,
                    ?headers,
                    ?body,
                    "invalidation request to Fastly failed"
                );

                Err(Error::PurgeFailed {
                    url,
                    status: Some(status),
                    source: error,
                })
            }
        }
    }

    fn token_header_value(&self) -> Result<HeaderValue, InvalidHeaderValue> {
        let api_token = self.api_token.expose_secret();

        let mut header_value = HeaderValue::try_from(api_token)?;
        header_value.set_sensitive(true);
        Ok(header_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Server, ServerOpts};

    const TEST_TOKEN: &str = "test-token";

    async fn mock_server() -> Server {
        Server::new_with_opts_async(ServerOpts {
            assert_on_drop: true,
            ..Default::default()
        })
        .await
    }

    fn client_with_server(server: &Server) -> Fastly {
        Fastly::with_api_base_url(TEST_TOKEN.to_string().into(), server.url())
    }

    #[tokio::test]
    async fn purges_path() {
        let mut server = mock_server().await;
        let _mock = server
            .mock(
                "POST",
                "/purge/static.crates.io/crates/serde/serde-1.0.0.crate",
            )
            .match_header("fastly-key", TEST_TOKEN)
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        client
            .purge("static.crates.io", "/crates/serde/serde-1.0.0.crate")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn purges_both_domains() {
        let mut server = mock_server().await;
        let _base_domain_mock = server
            .mock("POST", "/purge/static.crates.io/readmes/serde.html")
            .match_header("fastly-key", TEST_TOKEN)
            .with_status(200)
            .expect(1)
            .create_async()
            .await;
        let _prefixed_domain_mock = server
            .mock("POST", "/purge/fastly-static.crates.io/readmes/serde.html")
            .match_header("fastly-key", TEST_TOKEN)
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        client
            .purge_both_domains("static.crates.io", "/readmes/serde.html")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn rejects_wildcards() {
        let server = mock_server().await;
        let client = client_with_server(&server);

        let result = client.purge("static.crates.io", "/crates/*").await;

        std::assert_matches!(result, Err(Error::WildcardNotSupported));
    }

    #[tokio::test]
    async fn reports_http_errors() {
        let mut server = mock_server().await;
        let _mock = server
            .mock("POST", "/purge/static.crates.io/unavailable")
            .with_status(503)
            .expect(1)
            .create_async()
            .await;

        let client = client_with_server(&server);
        let result = client.purge("static.crates.io", "/unavailable").await;

        std::assert_matches!(
            result,
            Err(Error::PurgeFailed {
                status: Some(reqwest::StatusCode::SERVICE_UNAVAILABLE),
                ..
            })
        );
    }
}
