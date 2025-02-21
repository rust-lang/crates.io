use anyhow::{Context, anyhow};
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use secrecy::{ExposeSecret, SecretString};

#[derive(Debug)]
pub struct Fastly {
    client: Client,
    api_token: SecretString,
    static_domain_name: String,
}

impl Fastly {
    pub fn from_environment(client: Client) -> Option<Self> {
        let api_token = dotenvy::var("FASTLY_API_TOKEN").ok()?.into();
        let static_domain_name = dotenvy::var("S3_CDN").expect("missing S3_CDN");

        Some(Self {
            client,
            api_token,
            static_domain_name,
        })
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
    pub async fn invalidate(&self, path: &str) -> anyhow::Result<()> {
        if path.contains('*') {
            return Err(anyhow!(
                "wildcard invalidations are not supported for Fastly"
            ));
        }

        let domains = [
            &self.static_domain_name,
            &format!("fastly-{}", self.static_domain_name),
        ];
        let path = path.trim_start_matches('/');

        for domain in domains.iter() {
            let url = format!("https://api.fastly.com/purge/{}/{}", domain, path);
            self.purge_url(&url).await?;
        }

        Ok(())
    }

    async fn purge_url(&self, url: &str) -> anyhow::Result<()> {
        trace!(?url);

        let api_token = self.api_token.expose_secret();
        let mut api_token = HeaderValue::try_from(api_token)?;
        api_token.set_sensitive(true);

        let mut headers = HeaderMap::new();
        headers.append("Fastly-Key", api_token);

        debug!("sending invalidation request to Fastly");
        let response = self
            .client
            .post(url)
            .headers(headers)
            .send()
            .await
            .context("failed to send invalidation request to Fastly")?;

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

                Err(error).with_context(|| format!("failed to purge {url}"))
            }
        }
    }
}
