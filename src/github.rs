//! This module implements functionality for interacting with GitHub.

use oauth2::{prelude::*, AccessToken};
use reqwest::{self, header};

use serde::de::DeserializeOwned;

use std::str;

use crate::app::App;
use crate::util::errors::{cargo_err, internal, AppError, AppResult, NotFound};

/// Does all the nonsense for sending a GET to Github. Doesn't handle parsing
/// because custom error-code handling may be desirable. Use
/// `parse_github_response` to handle the "common" processing of responses.
pub fn github_api<T>(app: &App, url: &str, auth: &AccessToken) -> AppResult<T>
where
    T: DeserializeOwned,
{
    let url = format!("{}://api.github.com{}", app.config.api_protocol, url);
    info!("GITHUB HTTP: {}", url);

    app.http_client()
        .get(&url)
        .header(header::ACCEPT, "application/vnd.github.v3+json")
        .header(header::AUTHORIZATION, format!("token {}", auth.secret()))
        .header(header::USER_AGENT, "crates.io (https://crates.io)")
        .send()?
        .error_for_status()
        .map_err(|e| handle_error_response(&e))?
        .json()
        .map_err(Into::into)
}

fn handle_error_response(error: &reqwest::Error) -> Box<dyn AppError> {
    use reqwest::StatusCode as Status;

    match error.status() {
        Some(Status::UNAUTHORIZED) | Some(Status::FORBIDDEN) => cargo_err(
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

pub fn team_url(login: &str) -> String {
    let mut login_pieces = login.split(':');
    login_pieces.next();
    format!(
        "https://github.com/{}",
        login_pieces.next().expect("org failed"),
    )
}
