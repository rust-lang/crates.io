#![doc = include_str!("../README.md")]

use reqwest::Client;
use reqwest::header::{HeaderValue, InvalidHeaderValue};
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;
use tracing::{debug, instrument, trace};

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
}

impl Fastly {
    pub fn new(api_token: SecretString) -> Self {
        let client = Client::new();
        Self { client, api_token }
    }

    /// Invalidate a path on Fastly
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

    /// Invalidate a path on Fastly
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
        let url = format!("https://api.fastly.com/purge/{domain}/{path}");

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
