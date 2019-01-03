use url::Url;

use crate::background::{Builder, Runner};
use crate::git::{AddCrate, Yank};

pub fn job_runner(config: Builder<Environment>) -> Runner<Environment> {
    config.register::<AddCrate>().register::<Yank>().build()
}

#[allow(missing_debug_implementations)]
pub struct Environment {
    pub index_location: Url,
    pub credentials: Option<(String, String)>,
}

impl Environment {
    pub fn credentials(&self) -> Option<(&str, &str)> {
        self.credentials
            .as_ref()
            .map(|(u, p)| (u.as_str(), p.as_str()))
    }
}
