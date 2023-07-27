use reqwest::blocking::Client;
use std::fmt::Display;

pub struct ApiClient {
    http_client: Client,
}

impl ApiClient {
    pub fn new() -> anyhow::Result<Self> {
        let http_client = Client::builder()
            .user_agent("crates.io smoke test")
            .build()?;

        Ok(Self { http_client })
    }

    pub fn load_crate<N: Display>(&self, name: N) -> anyhow::Result<CrateResponse> {
        let url = format!(
            "https://staging.crates.io/api/v1/crates/{}?include=versions",
            name
        );

        self.http_client
            .get(url)
            .send()?
            .error_for_status()?
            .json()
            .map_err(Into::into)
    }

    pub fn load_version<N: Display, V: Display>(
        &self,
        name: N,
        version: V,
    ) -> anyhow::Result<VersionResponse> {
        let url = format!(
            "https://staging.crates.io/api/v1/crates/{}/{}",
            name, version
        );

        self.http_client
            .get(url)
            .send()?
            .error_for_status()?
            .json()
            .map_err(Into::into)
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct CrateResponse {
    #[serde(rename = "crate")]
    pub krate: Crate,
}

#[derive(Debug, serde::Deserialize)]
pub struct Crate {
    pub max_version: semver::Version,
}

#[derive(Debug, serde::Deserialize)]
pub struct VersionResponse {
    pub version: Version,
}

#[derive(Debug, serde::Deserialize)]
pub struct Version {
    #[serde(rename = "crate")]
    pub krate: String,
    pub num: semver::Version,
}
