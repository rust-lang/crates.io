//! Base configuration options
//!
//! - `HEROKU`: Is this instance of crates_io:: currently running on Heroku.

use crate::Env;
use crates_io_env_vars::var;

pub struct Base {
    pub env: Env,
}

impl Base {
    pub fn from_environment() -> anyhow::Result<Self> {
        let env = match var("HEROKU")? {
            Some(_) => Env::Production,
            _ => Env::Development,
        };

        Ok(Self { env })
    }
}
