use crate::worker::Environment;
use crates_io_worker::BackgroundJob;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const SQL_DIRECTORY: &str = "sql";

#[derive(Serialize, Deserialize)]
pub struct RunSql {
    file_name: String,
}

impl RunSql {
    pub fn new(file_name: String) -> Self {
        Self { file_name }
    }
}

impl BackgroundJob for RunSql {
    const JOB_NAME: &'static str = "run_sql";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    async fn run(&self, _env: Self::Context) -> anyhow::Result<()> {
        todo!();
    }
}
