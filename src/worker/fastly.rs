use anyhow::{anyhow, Context};
use reqwest::blocking::Client;

#[derive(Clone, Debug)]
pub struct Fastly {
    api_token: String,
    static_domain_name: String,
}

impl Fastly {
    pub fn from_environment() -> Option<Self> {
        let api_token = dotenvy::var("FASTLY_API_TOKEN").ok()?;
        let static_domain_name = dotenvy::var("S3_CDN").expect("missing S3_CDN");

        Some(Self {
            api_token,
            static_domain_name,
        })
    }

    /// Invalidate a path on Fastly
    ///
    /// This method takes a path and invalidates the cached content on Fastly. The path must not
    /// contain a wildcard, since the Fastly API does not support wildcard invalidations.
    ///
    /// Requests are authenticated using a token that is sent in a header. The token is passed to
    /// the application as an environment variable.
    ///
    /// More information on Fastly's APIs for cache invalidations can be found here:
    /// https://developer.fastly.com/reference/api/purging/
    #[instrument(skip(self, client))]
    pub fn invalidate(&self, client: &Client, path: &str) -> anyhow::Result<()> {
        if path.contains('*') {
            return Err(anyhow!(
                "wildcard invalidations are not supported for Fastly"
            ));
        }

        let path = path.trim_start_matches('/');
        let url = format!(
            "https://api.fastly.com/purge/{}/{}",
            self.static_domain_name, path
        );
        trace!(?url);

        let mut headers = reqwest::header::HeaderMap::new();
        headers.append("Fastly-Key", self.api_token.parse()?);

        debug!("sending invalidation request to Fastly");
        let response = client
            .post(&url)
            .headers(headers)
            .send()
            .context("failed to send invalidation request to Fastly")?;

        let status = response.status();

        match response.error_for_status_ref() {
            Ok(_) => {
                debug!(?status, "invalidation request accepted by Fastly");
                Ok(())
            }
            Err(error) => {
                let headers = response.headers().clone();
                let body = response.text();
                warn!(
                    ?status,
                    ?headers,
                    ?body,
                    "invalidation request to Fastly failed"
                );

                Err(error).with_context(|| format!("failed to invalidate {path} on Fastly"))
            }
        }
    }
}
