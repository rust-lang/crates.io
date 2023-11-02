use crate::{db, models::User};
use anyhow::anyhow;

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

pub fn run(opts: Opts) -> anyhow::Result<()> {
    let conn = &mut db::oneoff_connection()?;
    let user = User::find_by_api_token(conn, &opts.api_token).map_err(|err| anyhow!("{err}"))?;
    println!("The token belongs to user {}", user.gh_login);
    Ok(())
}
