use std::net::IpAddr;

use crates_io_env_vars::{var, var_parsed};

#[derive(Debug)]
pub struct BindConfig {
    /// IP address the server binds to. Defaults to `0.0.0.0` when running on
    /// Heroku or in the dev Docker container, and `127.0.0.1` otherwise.
    pub ip: IpAddr,

    /// Port the server binds to. Defaults to 8888.
    ///
    /// Read from the `PORT` environment variable.
    pub port: u16,
}

impl BindConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let docker = var("DEV_DOCKER")?.is_some();
        let heroku = var("HEROKU")?.is_some();

        let ip = if heroku || docker {
            [0, 0, 0, 0].into()
        } else {
            [127, 0, 0, 1].into()
        };

        let port = var_parsed("PORT")?.unwrap_or(8888);

        Ok(Self { ip, port })
    }
}
