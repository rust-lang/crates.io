use anyhow::Context;
use crates_io::models::ApiToken;
use crates_io::util::token::HashedToken;
use crates_io::{db, models::User};

#[derive(clap::Parser, Debug)]
#[command(
    name = "verify-token",
    about = "Look up a username by API token.",
    long_about = "Look up a username by API token. Used by staff to verify someone's identity \
        by having an API token given. If an error occurs, including being unable to \
        find a user with that API token, the error will be displayed."
)]
pub struct Opts {
    api_token: String,
}

pub async fn run(opts: Opts) -> anyhow::Result<()> {
    let mut conn = db::oneoff_connection()
        .await
        .context("Failed to connect to the database")?;

    let token = HashedToken::parse(&opts.api_token)?;
    let token = ApiToken::async_find_by_api_token(&mut conn, &token).await?;
    let user = User::async_find(&mut conn, token.user_id).await?;
    println!("The token belongs to user {}", user.gh_login);
    Ok(())
}
