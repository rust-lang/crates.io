//! This module implements functionality for interacting with GitHub.

use oauth2::AccessToken;
use reqwest::{self, header};

use serde::de::DeserializeOwned;

use std::str;

use crate::util::errors::{cargo_err, internal, not_found, AppError, AppResult};
use reqwest::blocking::Client;

#[derive(Debug)]
pub struct GitHubClient {
    base_url: String,
    client: Option<Client>,
}

impl GitHubClient {
    pub fn new(client: Option<Client>, base_url: String) -> Self {
        Self { client, base_url }
    }

    /// Does all the nonsense for sending a GET to Github. Doesn't handle parsing
    /// because custom error-code handling may be desirable. Use
    /// `parse_github_response` to handle the "common" processing of responses.
    pub fn request<T>(&self, url: &str, auth: &AccessToken) -> AppResult<T>
    where
        T: DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, url);
        info!("GITHUB HTTP: {}", url);

        self.client()
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

    /// Returns a client for making HTTP requests to upload crate files.
    ///
    /// The client will go through a proxy if the application was configured via
    /// `TestApp::with_proxy()`.
    ///
    /// # Panics
    ///
    /// Panics if the application was not initialized with a client.  This should only occur in
    /// tests that were not properly initialized.
    fn client(&self) -> &Client {
        self.client
            .as_ref()
            .expect("No HTTP client is configured.  In tests, use `TestApp::with_proxy()`.")
    }
}

fn handle_error_response(error: &reqwest::Error) -> Box<dyn AppError> {
    use reqwest::StatusCode as Status;

    match error.status() {
        Some(Status::UNAUTHORIZED) | Some(Status::FORBIDDEN) => cargo_err(
            "It looks like you don't have permission \
             to query a necessary property from GitHub \
             to complete this request. \
             You may need to re-authenticate on \
             crates.io to grant permission to read \
             GitHub org memberships.",
        ),
        Some(Status::NOT_FOUND) => not_found(),
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
