use crate::models::ApiToken;
use crate::tasks::spawn_blocking;
use crate::util::token::HashedToken;
use crate::{db, models::User};

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
    spawn_blocking(move || {
        let conn = &mut db::oneoff_connection()?;
        let token = HashedToken::parse(&opts.api_token)?;
        let token = ApiToken::find_by_api_token(conn, &token)?;
        let user = User::find(conn, token.user_id)?;
        println!("The token belongs to user {}", user.gh_login);
        Ok(())
    })
    .await
}
