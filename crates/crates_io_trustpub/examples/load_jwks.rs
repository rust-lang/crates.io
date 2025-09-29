use clap::{Parser, ValueEnum};
use crates_io_trustpub::github::GITHUB_ISSUER_URL;
use crates_io_trustpub::keystore::load_jwks::load_jwks;
use reqwest::Client;

#[derive(Clone, Debug, ValueEnum)]
enum Provider {
    #[value(name = "github")]
    GitHub,
}

impl Provider {
    fn issuer_url(&self) -> &'static str {
        match self {
            Provider::GitHub => GITHUB_ISSUER_URL,
        }
    }
}

#[derive(Parser)]
#[command(name = "load_jwks")]
#[command(about = "Load and display JWKS keys from OpenID Connect providers")]
struct Args {
    /// The provider to load JWKS from
    provider: Provider,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let issuer_url = args.provider.issuer_url();

    println!("Loading JWKS for provider: {:?}", args.provider);
    println!("Issuer URL: {}", issuer_url);
    println!();

    let client = Client::new();

    let jwks = load_jwks(&client, issuer_url).await?;
    println!("Successfully loaded JWKS with {} keys:", jwks.keys.len());
    println!();

    for key in &jwks.keys {
        let key_id = key.common.key_id.as_deref().unwrap_or("<none>");
        println!("Key ID: {}", key_id);

        if let Some(alg) = &key.common.key_algorithm {
            println!("Algorithm: {:?}", alg);
        }

        if let Some(usage) = &key.common.public_key_use {
            println!("Usage: {:?}", usage);
        }

        println!("Algorithm Parameters: {:?}", key.algorithm);

        println!();
    }

    Ok(())
}
