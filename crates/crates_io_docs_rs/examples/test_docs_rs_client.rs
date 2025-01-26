use anyhow::{Result, anyhow};
use crates_io_docs_rs::{DocsRsClient, RealDocsRsClient};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let access_token = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("Missing access token"))?;

    let docs_rs = RealDocsRsClient::new("https://docs.rs", access_token)?;

    docs_rs.rebuild_docs("empty-library", "1.0.0").await?;

    Ok(())
}
