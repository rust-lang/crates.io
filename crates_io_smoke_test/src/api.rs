use bytes::Bytes;
use crates_io_index::Repository;
use reqwest::Client;
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

    pub async fn load_crate<N: Display>(&self, name: N) -> anyhow::Result<CrateResponse> {
        let url = format!(
            "https://staging.crates.io/api/v1/crates/{}?include=versions",
            name
        );

        let response = self.http_client.get(url).send().await?;
        let response = response.error_for_status()?;
        response.json().await.map_err(Into::into)
    }

    pub async fn load_version<N: Display, V: Display>(
        &self,
        name: N,
        version: V,
    ) -> anyhow::Result<VersionResponse> {
        let url = format!(
            "https://staging.crates.io/api/v1/crates/{}/{}",
            name, version
        );

        let response = self.http_client.get(url).send().await?;
        let response = response.error_for_status()?;
        response.json().await.map_err(Into::into)
    }

    pub async fn download_crate_file_via_api<N: Display, V: Display>(
        &self,
        name: N,
        version: V,
    ) -> anyhow::Result<Bytes> {
        let url = format!(
            "https://staging.crates.io/api/v1/crates/{}/{}/download",
            name, version
        );

        let response = self.http_client.get(url).send().await?;
        let response = response.error_for_status()?;
        response.bytes().await.map_err(Into::into)
    }

    pub async fn download_crate_file_via_cdn<N: Display, V: Display>(
        &self,
        name: N,
        version: V,
    ) -> anyhow::Result<Bytes> {
        let url = format!(
            "https://static.staging.crates.io/crates/{}/{}/download",
            name, version
        );

        let response = self.http_client.get(url).send().await?;
        let response = response.error_for_status()?;
        response.bytes().await.map_err(Into::into)
    }

    pub async fn load_from_sparse_index(
        &self,
        name: &str,
    ) -> anyhow::Result<Vec<crates_io_index::Crate>> {
        let path = Repository::relative_index_file_for_url(name);

        let url = format!("https://index.staging.crates.io/{path}",);

        let response = self.http_client.get(url).send().await?;
        let response = response.error_for_status()?;
        let text = response.text().await?;

        text.lines()
            .map(|line| serde_json::from_str(line).map_err(Into::into))
            .collect()
    }

    pub async fn load_from_git_index(
        &self,
        name: &str,
    ) -> anyhow::Result<Vec<crates_io_index::Crate>> {
        let path = Repository::relative_index_file_for_url(name);

        let url = format!(
            "https://raw.githubusercontent.com/rust-lang/staging.crates.io-index/master/{path}",
        );

        let response = self.http_client.get(url).send().await?;
        let response = response.error_for_status()?;
        let text = response.text().await?;

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
