use curl;
use oauth2::*;

pub fn github(url: &str, auth: &Token)
    -> Result<curl::http::Response, curl::ErrCode> {
    curl::http::handle()
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
