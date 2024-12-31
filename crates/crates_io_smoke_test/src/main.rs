#![doc = include_str!("../README.md")]

mod api;
mod cargo;
mod exit_status_ext;
mod git;

#[macro_use]
extern crate tracing;

use crate::api::ApiClient;
use anyhow::{anyhow, bail, Context};
use clap::Parser;
use secrecy::SecretString;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

#[derive(clap::Parser, Debug)]
struct Options {
    /// name of the test crate that will be published to staging.crates.io
    #[arg(long, default_value = "crates-staging-test-tb")]
    crate_name: String,

    /// staging.crates.io API token that will be used to publish a new version
    #[arg(long, env = "CARGO_REGISTRY_TOKEN", hide_env_values = true)]
    token: SecretString,

    /// skip the publishing step and run the verifications for the highest
    /// uploaded version instead.
    #[arg(long)]
    skip_publish: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let options = Options::parse();
    debug!(?options);

    let api_client = ApiClient::new().context("Failed to initialize API client")?;

    info!("Loading crate information from staging.crates.io…");
    let krate = api_client
        .load_crate(&options.crate_name)
        .await
        .context("Failed to load crate information from staging.crates.io")?
        .krate;

    let old_version = krate.max_version;
    let mut new_version = old_version.clone();

    if !options.skip_publish {
        new_version.patch += 1;
        info!(%old_version, %new_version, "Calculated new version number");
    } else {
        info!(%old_version, %new_version, "Using old version number since `--skip-publish` is set");
    }

    info!("Creating temporary working folder…");
    let tempdir = tempdir().context("Failed to create temporary working folder")?;
    debug!(tempdir.path = %tempdir.path().display());

    info!("Creating `{}` project…", options.crate_name);
    let project_path = create_project(tempdir.path(), &options.crate_name, &new_version)
        .await
        .context("Failed to create project")?;

    if options.skip_publish {
        info!("Packaging crate file…");
        cargo::package(&project_path)
            .await
            .context("Failed to run `cargo package`")?;

        info!("Skipping publish step");
    } else {
        info!("Publishing to staging.crates.io…");
        cargo::publish(&project_path, &options.token)
            .await
            .context("Failed to run `cargo publish`")?;
    }

    let version = new_version;
    info!(%version, "Checking staging.crates.io API for the new version…");

    let json = api_client
        .load_version(&options.crate_name, &version)
        .await
        .context("Failed to load version information from staging.crates.io")?;

    if json.version.krate != options.crate_name {
        return Err(anyhow!(
            "API returned an unexpected crate name; expected `{}` found `{}`",
            options.crate_name,
            json.version.krate
        ));
    }

    if json.version.num != version {
        return Err(anyhow!(
            "API returned an unexpected version number; expected `{}` found `{}`",
            version,
            json.version.num
        ));
    }

    info!(%version, "Checking crate file download from staging.crates.io API…");

    let bytes = api_client
        .download_crate_file_via_api(&options.crate_name, &version)
        .await
        .context("Failed to download crate file")?;

    if bytes.len() < 500 {
        return Err(anyhow!(
            "API returned an unexpectedly small crate file; size: {}",
            bytes.len()
        ));
    }
    info!(%version, "Checking crate file download from static.staging.crates.io CDN…");

    let bytes = api_client
        .download_crate_file_via_cdn(&options.crate_name, &version)
        .await
        .context("Failed to download crate file")?;

    if bytes.len() < 500 {
        return Err(anyhow!(
            "CDN returned an unexpectedly small crate file; size: {}",
            bytes.len()
        ));
    }

    info!("Checking sparse index…");
    let sparse_index_records = api_client
        .load_from_sparse_index(&options.crate_name)
        .await
        .context("Failed to load sparse index data")?;

    let version_str = version.to_string();
    let record = sparse_index_records
        .iter()
        .find(|record| record.vers == version_str);
    if record.is_none() {
        return Err(anyhow!(
            "Failed to find published version on the sparse index"
        ));
    }

    info!("Checking git index…");
    let git_index_records = api_client
        .load_from_git_index(&options.crate_name)
        .await
        .context("Failed to load git index data")?;

    let record = git_index_records
        .iter()
        .find(|record| record.vers == version_str);
    if record.is_none() {
        return Err(anyhow!("Failed to find published version on the git index"));
    }

    if !options.skip_publish {
        info!("Checking failed publish with large payload…");
        create_dummy_content(&project_path).await?;

        info!("Sending publish request…");
        let output = cargo::publish_with_output(&project_path, &options.token).await?;
        if output.status.success() {
            bail!("Expected `cargo publish` to fail with invalid token");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("413 Payload Too Large") {
                bail!("Expected `cargo publish` to fail with an `413 Payload Too Large` error, but got:\n{stderr}");
            }
        }
    }

    info!(
        "All automated smoke tests have passed.\n\nPlease visit https://staging.crates.io/crates/{}/{} for further manual testing.",
        &options.crate_name, &version
    );

    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    let log_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_filter(env_filter);

    tracing_subscriber::registry().with(log_layer).init();
}

async fn create_project(
    parent_path: &Path,
    name: &str,
    version: &semver::Version,
) -> anyhow::Result<PathBuf> {
    let version = version.to_string();

    cargo::new_lib(parent_path, name)
        .await
        .context("Failed to run `cargo new`")?;

    let project_path = parent_path.join(name);
    debug!(project_path = %project_path.display());

    write_manifest(&project_path, name, &version).await?;

    {
        let readme_path = project_path.join("README.md");
        info!(readme_path = %readme_path.display(), "Creating `README.md` file…");

        let new_content = format!(
            "# {name} v{version}\n\n![](https://media1.giphy.com/media/Ju7l5y9osyymQ/200.gif)\n",
        );

        fs::write(&readme_path, new_content)
            .await
            .context("Failed to write `README.md` file content")?;
    }

    info!("Creating initial git commit…");
    git::set_user_name(&project_path, "crates-io-smoke-test")
        .await
        .context("Failed to set git user name")?;

    git::set_user_email(&project_path, "smoke-test@crates.io")
        .await
        .context("Failed to set git user email")?;

    git::add_all(&project_path)
        .await
        .context("Failed to add initial changes to git")?;

    git::commit(&project_path, "initial commit")
        .await
        .context("Failed to commit initial changes")?;

    Ok(project_path)
}

async fn write_manifest(project_path: &Path, name: &str, version: &str) -> anyhow::Result<()> {
    let manifest_path = project_path.join("Cargo.toml");
    info!(manifest_path = %manifest_path.display(), "Overriding `Cargo.toml` file…");

    let new_content = format!(
        r#"[package]
name = "{name}"
version = "{version}"
edition = "2018"
license = "MIT"
description = "test crate"
"#,
    );

    fs::write(&manifest_path, new_content)
        .await
        .context("Failed to write `Cargo.toml` file content")?;

    Ok(())
}

async fn create_dummy_content(project_path: &Path) -> anyhow::Result<()> {
    const FILE_SIZE: u32 = 15 * 1024 * 1024;

    debug!("Creating `dummy.txt` file…");
    let f = fs::File::create(project_path.join("dummy.txt")).await?;
    let mut writer = tokio::io::BufWriter::new(f);
    for _ in 0..(FILE_SIZE / 16) {
        writer.write_u128(rand::random()).await?;
    }
    drop(writer);

    write_manifest(project_path, "dummy", "0.0.0-dummy").await?;

    debug!("Creating additional git commit…");
    git::add_all(project_path)
        .await
        .context("Failed to add changes to git")?;

    git::commit(project_path, "add dummy content")
        .await
        .context("Failed to commit changes")?;

    Ok(())
}
