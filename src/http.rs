use curl;
use oauth2::*;
use app::App;

/// Does all the nonsense for sending a GET to Github
pub fn github(app: &App, url: &str, auth: &Token)
    -> Result<curl::http::Response, curl::ErrCode> {
    println!("HTTP: {}", url);
    app.handle()
     .get(url)
     .header("Accept", "application/vnd.github.v3+json")
     .header("User-Agent", "hello!")
     .auth_with(auth)
     .exec()
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
