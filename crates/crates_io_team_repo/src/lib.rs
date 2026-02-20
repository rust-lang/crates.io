#![doc = include_str!("../README.md")]

use async_trait::async_trait;
use reqwest::{Certificate, Client};
use serde::Deserialize;

mod certs;

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait TeamRepo {
    async fn get_permission(&self, name: &str) -> anyhow::Result<Permission>;
}

#[derive(Debug, Clone, Deserialize)]
pub struct Permission {
    pub people: Vec<Person>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Person {
    pub name: String,
    pub github: String,
    pub github_id: i64,
}

pub struct TeamRepoImpl {
    client: Client,
}

impl TeamRepoImpl {
    fn new(client: Client) -> Self {
        TeamRepoImpl { client }
    }
}

impl Default for TeamRepoImpl {
    fn default() -> Self {
        let client = build_client();
        TeamRepoImpl::new(client)
    }
}

fn build_client() -> Client {
    let x1_cert = Certificate::from_pem(certs::ISRG_ROOT_X1).unwrap();
    let x2_cert = Certificate::from_pem(certs::ISRG_ROOT_X2).unwrap();

    Client::builder()
        .tls_certs_only([x1_cert, x2_cert])
        .build()
        .unwrap()
}

#[async_trait]
impl TeamRepo for TeamRepoImpl {
    async fn get_permission(&self, name: &str) -> anyhow::Result<Permission> {
        let url = format!("https://team-api.infra.rust-lang.org/v1/permissions/{name}.json");
        let response = self.client.get(url).send().await?.error_for_status()?;
        Ok(response.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::build_client;

    /// This test is here to make sure that the client is built
    /// correctly without panicking.
    #[test]
    fn test_build_client() {
        let _client = build_client();
    }
}
