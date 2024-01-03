//! The code in this module interacts with the
//! <https://github.com/rust-lang/team/> repository.
//!
//! The [TeamRepo] trait is used to abstract away the HTTP client for testing
//! purposes. The [TeamRepoImpl] struct is the actual implementation of
//! the trait.

use async_trait::async_trait;
use mockall::automock;
use reqwest::Client;

#[automock]
#[async_trait]
pub trait TeamRepo {
    async fn get_team(&self, name: &str) -> anyhow::Result<Team>;
}

#[derive(Debug, Clone, Deserialize)]
pub struct Team {
    pub name: String,
    pub kind: String,
    pub members: Vec<Member>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Member {
    pub name: String,
    pub github: String,
    pub github_id: i32,
    pub is_lead: bool,
}

pub struct TeamRepoImpl {
    client: Client,
}

impl TeamRepoImpl {
    pub fn new(client: Client) -> Self {
        TeamRepoImpl { client }
    }
}

#[async_trait]
impl TeamRepo for TeamRepoImpl {
    async fn get_team(&self, name: &str) -> anyhow::Result<Team> {
        let url = format!("https://team-api.infra.rust-lang.org/v1/teams/{name}.json");
        let response = self.client.get(url).send().await?.error_for_status()?;
        Ok(response.json().await?)
    }
}
