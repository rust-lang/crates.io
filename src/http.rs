use curl;
use oauth2::*;
use app::App;

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

pub fn token(token: String) -> Token {
    Token {
        access_token: token,
        scopes: Vec::new(),
        token_type: String::new(),
    }
}
