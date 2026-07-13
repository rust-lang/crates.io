use std::str::FromStr;

use clap::Parser;
use crates_io_fastly::Fastly;
use secrecy::SecretString;

/// Arguments accepted by the purge example.
#[derive(Debug, Parser)]
#[command(about = "Purge cached content using the crates.io Fastly client")]
struct Options {
    /// Fastly API token used to authenticate the purge request.
    #[arg(long, env = "FASTLY_API_TOKEN", hide_env_values = true)]
    api_token: SecretString,

    /// URL to purge, without the scheme.
    #[arg(long, value_name = "DOMAIN/PATH")]
    url: PurgeUrl,
}

/// Domain and path extracted from a URL argument.
#[derive(Clone, Debug, PartialEq)]
struct PurgeUrl {
    domain: String,
    path: String,
}

impl FromStr for PurgeUrl {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.starts_with("http://") || value.starts_with("https://") {
            return Err("URL must omit the `http://` or `https://` scheme");
        }

        let (domain, path) = value
            .split_once('/')
            .ok_or("URL must contain a domain and path")?;

        if domain.is_empty() {
            return Err("URL domain must not be empty");
        }

        Ok(Self {
            domain: domain.into(),
            path: path.into(),
        })
    }
}

/// Sends the requested purge through the Fastly API.
#[tokio::main]
async fn main() -> Result<(), crates_io_fastly::Error> {
    let options = Options::parse();
    let fastly = Fastly::new(options.api_token);

    fastly.purge(&options.url.domain, &options.url.path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_url_with_http_scheme() {
        for url in [
            "http://static.crates.io/crates/serde/serde-1.0.0.crate",
            "https://static.crates.io/crates/serde/serde-1.0.0.crate",
        ] {
            assert_eq!(
                url.parse::<PurgeUrl>(),
                Err("URL must omit the `http://` or `https://` scheme")
            );
        }
    }

    #[test]
    fn parses_url_purge() {
        let options = Options::try_parse_from([
            "purge",
            "--api-token",
            "test-token",
            "--url",
            "static.crates.io/crates/serde/serde-1.0.0.crate",
        ])
        .unwrap();

        assert_eq!(
            options.url,
            PurgeUrl {
                domain: "static.crates.io".into(),
                path: "crates/serde/serde-1.0.0.crate".into(),
            }
        );
    }
}
