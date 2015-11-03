use curl;
use oauth2::*;
use app::App;
use util::{CargoResult, internal, ChainError, human};
use rustc_serialize::{json, Decodable};
use std::str;


/// Does all the nonsense for sending a GET to Github. Doesn't handle parsing
/// because custom error-code handling may be desirable. Use
/// parse_github_response to handle the "common" processing of responses.
pub fn github(app: &App, url: &str, auth: &Token)
              -> Result<curl::http::Response, curl::ErrCode>
{
    let url = format!("{}://api.github.com{}", app.config.api_protocol(), url);
    info!("GITHUB HTTP: {}", url);

    app.handle()
       .get(url)
       .header("Accept", "application/vnd.github.v3+json")
       .header("User-Agent", "hello!")
       .auth_with(auth)
       .exec()
}

/// Checks for normal responses
pub fn parse_github_response<T: Decodable>(resp: curl::http::Response)
                                            -> CargoResult<T> {
    match resp.get_code() {
        200 => {} // Ok!
        403 => {
            return Err(human("It looks like you don't have permission \
                              to query a necessary property from Github \
                              to complete this request. \
                              You may need to re-authenticate on \
                              crates.io to grant permission to read \
                              github org memberships. Just go to \
                              https://crates.io/login"));
        }
        _ => {
            return Err(internal(format!("didn't get a 200 result from
                                        github: {}", resp)));
        }
    }

    let json = try!(str::from_utf8(resp.get_body()).ok().chain_error(||{
        internal("github didn't send a utf8-response")
    }));

    json::decode(json).chain_error(|| {
        internal("github didn't send a valid json response")
    })
}

/// Gets a token with the given string as the access token, but all
/// other info null'd out. Generally, just to be fed to the `github` fn.
pub fn token(token: String) -> Token {
    Token {
        access_token: token,
        scopes: Vec::new(),
        token_type: String::new(),
    }
}
