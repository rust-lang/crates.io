use bytes::Bytes;
use crates_io_index::Repository;
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

    pub fn download_crate_file<N: Display, V: Display>(
        &self,
        name: N,
        version: V,
    ) -> anyhow::Result<Bytes> {
        let url = format!(
            "https://staging.crates.io/api/v1/crates/{}/{}/download",
            name, version
        );

        self.http_client
            .get(url)
            .send()?
            .error_for_status()?
            .bytes()
            .map_err(Into::into)
    }

    pub fn load_from_sparse_index(
        &self,
        name: &str,
    ) -> anyhow::Result<Vec<crates_io_index::Crate>> {
        let path = Repository::relative_index_file_for_url(name);

        let url = format!("https://index.staging.crates.io/{path}",);

        let text = self
            .http_client
            .get(url)
            .send()?
            .error_for_status()?
            .text()?;

        text.lines()
            .map(|line| serde_json::from_str(line).map_err(Into::into))
            .collect()
    }

    pub fn load_from_git_index(&self, name: &str) -> anyhow::Result<Vec<crates_io_index::Crate>> {
        let path = Repository::relative_index_file_for_url(name);

        let url = format!(
            "https://raw.githubusercontent.com/rust-lang/staging.crates.io-index/master/{path}",
        );

        let text = self
            .http_client
            .get(url)
            .send()?
            .error_for_status()?
            .text()?;

        text.lines()
            .map(|line| serde_json::from_str(line).map_err(Into::into))
            .collect()
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
