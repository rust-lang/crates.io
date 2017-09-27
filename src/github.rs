//! This module implements functionality for interacting with GitHub.

use curl;
use curl::easy::{Easy, List};

use oauth2::*;

use serde_json;
use serde::Deserialize;

use std::str;

use app::App;
use util::{human, internal, CargoResult, ChainError};

/// Does all the nonsense for sending a GET to Github. Doesn't handle parsing
/// because custom error-code handling may be desirable. Use
/// `parse_github_response` to handle the "common" processing of responses.
pub fn github(app: &App, url: &str, auth: &Token) -> Result<(Easy, Vec<u8>), curl::Error> {
    let url = format!("{}://api.github.com{}", app.config.api_protocol, url);
    info!("GITHUB HTTP: {}", url);

    let mut headers = List::new();
    headers
        .append("Accept: application/vnd.github.v3+json")
        .unwrap();
    headers.append("User-Agent: hello!").unwrap();
    headers
        .append(&format!("Authorization: token {}", auth.access_token))
        .unwrap();

    let mut handle = app.handle();
    handle.url(&url).unwrap();
    handle.get(true).unwrap();
    handle.http_headers(headers).unwrap();

    let mut data = Vec::new();
    {
        let mut transfer = handle.transfer();
        transfer
            .write_function(|buf| {
                data.extend_from_slice(buf);
                Ok(buf.len())
            })
            .unwrap();
        transfer.perform()?;
    }
    Ok((handle, data))
}

/// Checks for normal responses
pub fn parse_github_response<'de, 'a: 'de, T: Deserialize<'de>>(
    mut resp: Easy,
    data: &'a [u8],
) -> CargoResult<T> {
    match resp.response_code().unwrap() {
        200 => {}
        // Ok!
        403 => {
            return Err(human(
                "It looks like you don't have permission \
                 to query a necessary property from Github \
                 to complete this request. \
                 You may need to re-authenticate on \
                 crates.io to grant permission to read \
                 github org memberships. Just go to \
                 https://crates.io/login",
            ));
        }
        n => {
            let resp = String::from_utf8_lossy(data);
            return Err(internal(&format_args!(
                "didn't get a 200 result from \
                 github, got {} with: {}",
                n,
                resp
            )));
        }
    }

    let json = str::from_utf8(data)
        .ok()
        .chain_error(|| internal("github didn't send a utf8-response"))?;

    serde_json::from_str(json).chain_error(|| internal("github didn't send a valid json response"))
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
