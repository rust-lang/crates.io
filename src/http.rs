use conduit::{Request, Response};
use conduit_middleware::Middleware;

use curl;
use curl::easy::{Easy, List};

use oauth2::*;

use serde_json;
use serde::Deserialize;

use std::str;
use std::error::Error;
use std::collections::HashMap;

use app::App;
use util::{CargoResult, internal, ChainError, human};
use Uploader;

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
        200 => {} // Ok!
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

    let json = str::from_utf8(data).ok().chain_error(|| {
        internal("github didn't send a utf8-response")
    })?;

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

#[derive(Clone, Debug)]
pub struct SecurityHeadersMiddleware {
    headers: HashMap<String, Vec<String>>,
}

impl SecurityHeadersMiddleware {
    pub fn new(uploader: &Uploader) -> Self {
        let mut headers = HashMap::new();

        headers.insert(
            "X-Content-Type-Options".into(),
            vec!["nosniff".into()],
        );

        headers.insert(
            "X-Frame-Options".into(),
            vec!["SAMEORIGIN".into()],
        );

        headers.insert(
            "X-XSS-Protection".into(),
            vec!["1; mode=block".into()],
        );

        let s3_host = match *uploader {
            Uploader::S3 { ref bucket, .. } => bucket.host(),
            _ => unreachable!("This middleware should only be used in the production environment, \
                               which should also require an S3 uploader, QED"),
        };

        // It would be better if we didn't have to have 'unsafe-eval' in the `script-src`
        // policy, but google charts (used for the download graph on crate pages) uses `eval`
        // to load scripts. Remove 'unsafe-eval' if google fixes the issue:
        // https://github.com/google/google-visualization-issues/issues/1356
        // or if we switch to a different graph generation library.
        headers.insert(
            "Content-Security-Policy".into(),
            vec![
                format!("default-src 'self'; \
                  connect-src 'self' https://docs.rs https://{}; \
                  script-src 'self' 'unsafe-eval' \
                             https://www.google-analytics.com https://www.google.com; \
                  style-src 'self' https://www.google.com https://ajax.googleapis.com; \
                  img-src *; \
                  object-src 'none'",
                  s3_host
                ),
            ],
        );

        SecurityHeadersMiddleware { headers }
    }
}

impl Middleware for SecurityHeadersMiddleware {
    fn after(
        &self,
        _: &mut Request,
        mut res: Result<Response, Box<Error + Send>>,
    ) -> Result<Response, Box<Error + Send>> {
        if let Ok(ref mut response) = res {
            response.headers.extend(self.headers.clone());
        }
        res
    }
}
