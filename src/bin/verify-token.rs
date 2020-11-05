use cargo_registry::{db, models::User, util::errors::AppResult};

use clap::Clap;

#[derive(Clap, Debug)]
#[clap(
    name = "verify-token",
    about = "Look up a username by API token.",
    long_about = "Look up a username by API token. Used by staff to verify someone's identity \
        by having an API token given. If an error occurs, including being unable to \
        find a user with that API token, the error will be displayed."
)]
struct Opts {
    api_token: String,
}

fn main() -> AppResult<()> {
    let opts: Opts = Opts::parse();

    let conn = db::connect_now()?;
    let user = User::find_by_api_token(&conn, &opts.api_token)?;
    println!("The token belongs to user {}", user.gh_login);
    Ok(())
}
