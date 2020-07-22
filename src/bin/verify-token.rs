// Look up a username by API token. Used by staff to verify someone's identity
// by having an API token given. If an error occurs, including being unable to
// find a user with that API token, the error will be displayed.

use cargo_registry::{db, models::User, util::errors::AppResult};
use std::env;

fn main() -> AppResult<()> {
    let conn = db::connect_now()?;
    let token = env::args().nth(1).expect("API token argument required");
    let user = User::find_by_api_token(&conn, &token)?;
    println!("The token belongs to user {}", user.gh_login);
    Ok(())
}
