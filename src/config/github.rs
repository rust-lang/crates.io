use oauth2::{ClientId, ClientSecret};

use crates_io_env_vars::required_var;

#[derive(Debug)]
pub struct GitHubOAuthConfig {
    /// The client ID of the associated GitHub application.
    ///
    /// Read from the `GH_CLIENT_ID` environment variable.
    pub client_id: ClientId,

    /// The client secret of the associated GitHub application.
    ///
    /// Read from the `GH_CLIENT_SECRET` environment variable.
    pub client_secret: ClientSecret,
}

impl GitHubOAuthConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let client_id = ClientId::new(required_var("GH_CLIENT_ID")?);
        let client_secret = ClientSecret::new(required_var("GH_CLIENT_SECRET")?);

        Ok(Self {
            client_id,
            client_secret,
        })
    }
}
