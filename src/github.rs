//! This module implements functionality for interacting with GitHub.

use oauth2::*;
use reqwest::{self, header};

use serde::de::DeserializeOwned;

use std::str;

use crate::app::App;
use crate::util::{errors::NotFound, human, internal, CargoError, CargoResult};

/// Does all the nonsense for sending a GET to Github. Doesn't handle parsing
/// because custom error-code handling may be desirable. Use
/// `parse_github_response` to handle the "common" processing of responses.
pub fn github<T>(app: &App, url: &str, auth: &Token) -> CargoResult<T>
where
    T: DeserializeOwned,
{
    let url = format!("{}://api.github.com{}", app.config.api_protocol, url);
    info!("GITHUB HTTP: {}", url);

    app.http_client()
        .get(&url)
        .header(header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            header::AUTHORIZATION,
            format!("token {}", auth.access_token),
        )
        .send()?
        .error_for_status()
        .map_err(|e| handle_error_response(&e))?
        .json()
        .map_err(Into::into)
}

fn handle_error_response(error: &reqwest::Error) -> Box<dyn CargoError> {
    use reqwest::StatusCode as Status;

    match error.status() {
        Some(Status::UNAUTHORIZED) | Some(Status::FORBIDDEN) => human(
            "It looks like you don't have permission \
             to query a necessary property from Github \
             to complete this request. \
             You may need to re-authenticate on \
             crates.io to grant permission to read \
             github org memberships. Just go to \
             https://crates.io/login",
        ),
        Some(Status::NOT_FOUND) => Box::new(NotFound),
        _ => internal(&format_args!(
            "didn't get a 200 result from github: {}",
            error
        )),
    }
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

pub fn team_url(login: &str) -> String {
    let mut login_pieces = login.split(':');
    login_pieces.next();
    format!(
        "https://github.com/{}",
        login_pieces.next().expect("org failed"),
    )
}
