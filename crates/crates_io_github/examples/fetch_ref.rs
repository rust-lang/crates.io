use anyhow::Result;
use clap::Parser;
use crates_io_github::{GitHubAuth, GitHubClient, RealGitHubClient, parse_github_slug};
use reqwest::Client;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use url::Url;

#[derive(Debug, Parser)]
#[command(about = "Prints the head commit and tree SHAs for a branch of a GitHub repo.")]
struct Opts {
    /// GitHub repository URL, e.g. `https://github.com/rust-lang/crates.io-index`.
    repo: Url,

    /// Branch to resolve. Defaults to `master` because that is what the
    /// crates.io index uses; pass `--branch main` (or similar) for other
    /// repositories.
    #[arg(long, default_value = "master")]
    branch: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let opts = Opts::parse();
    let (owner, repo) = parse_github_slug(&opts.repo)?;
    let ref_name = format!("refs/heads/{}", opts.branch);

    let client = RealGitHubClient::new(Client::new());
    let auth = GitHubAuth::None;
    let git_ref = client.get_ref(&owner, &repo, &ref_name, &auth).await?;
    let commit = client
        .get_commit(&owner, &repo, &git_ref.object.sha, &auth)
        .await?;

    println!("ref:      {}", git_ref.ref_name);
    println!("commit:   {}", commit.sha);
    println!("tree:     {}", commit.tree.sha);
    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy();

    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .init();
}
