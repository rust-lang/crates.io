use std::str::FromStr;

use clap::{ArgGroup, Parser};
use crates_io_fastly::Fastly;
use secrecy::SecretString;

/// Arguments accepted by the purge example.
#[derive(Debug, Parser)]
#[command(
    about = "Purge cached content using the crates.io Fastly client",
    group(
        ArgGroup::new("operation")
            .required(true)
            .args(["url", "service_id"])
    )
)]
struct Options {
    /// Fastly API token used to authenticate the purge request.
    #[arg(long, env = "FASTLY_API_TOKEN", hide_env_values = true)]
    api_token: SecretString,

    /// URL to purge, without the scheme.
    #[arg(long, value_name = "DOMAIN/PATH")]
    url: Option<PurgeUrl>,

    /// Fastly service containing the objects associated with the key.
    #[arg(long, value_name = "ID", requires = "key")]
    service_id: Option<String>,

    /// Surrogate key to purge.
    #[arg(value_name = "KEY", requires = "service_id")]
    key: Option<String>,
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

    match (options.url, options.service_id, options.key) {
        (Some(url), None, None) => fastly.purge(&url.domain, &url.path).await,
        (None, Some(service_id), Some(key)) => fastly.purge_surrogate_key(&service_id, &key).await,
        _ => unreachable!("clap validates exactly one complete purge operation"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

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
            Some(PurgeUrl {
                domain: "static.crates.io".into(),
                path: "crates/serde/serde-1.0.0.crate".into(),
            })
        );
        assert_eq!(options.service_id, None);
        assert_eq!(options.key, None);
    }

    #[test]
    fn parses_surrogate_key_purge() {
        let options = Options::try_parse_from([
            "purge",
            "--api-token",
            "test-token",
            "--service-id",
            "static-service-id",
            "release:serde@1.0.0+metadata",
        ])
        .unwrap();

        assert_eq!(options.url, None);
        assert_eq!(options.service_id.as_deref(), Some("static-service-id"));
        assert_eq!(options.key.as_deref(), Some("release:serde@1.0.0+metadata"));
    }

    #[test]
    fn rejects_service_id_without_key() {
        let error = Options::try_parse_from([
            "purge",
            "--api-token",
            "test-token",
            "--service-id",
            "static-service-id",
        ])
        .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn rejects_multiple_operations() {
        let error = Options::try_parse_from([
            "purge",
            "--api-token",
            "test-token",
            "--url",
            "static.crates.io/crates/serde/serde-1.0.0.crate",
            "--service-id",
            "static-service-id",
            "crate:serde",
        ])
        .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
    }
}
