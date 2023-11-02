//! Base configuration options
//!
//! - `HEROKU`: Is this instance of crates_io:: currently running on Heroku.

use crate::Env;

pub struct Base {
    pub env: Env,
}

impl Base {
    pub fn from_environment() -> anyhow::Result<Self> {
        let env = match dotenvy::var("HEROKU") {
            Ok(_) => Env::Production,
            _ => Env::Development,
        };

        Ok(Self { env })
    }
}
