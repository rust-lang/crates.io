use crate::controllers;
use crate::controllers::util::RequestPartsExt;
use crate::middleware::log_request::RequestLogExt;
use crate::middleware::session::RequestSession;
use crate::models::token::{CrateScope, EndpointScope};
use crate::models::{ApiToken, User};
use crate::util::diesel::Conn;
use crate::util::errors::{
    account_locked, forbidden, internal, AppResult, InsecurelyGeneratedTokenRevoked,
};
use crate::util::token::HashedToken;
use chrono::Utc;
use http::header;
use http::request::Parts;

#[derive(Debug, Clone)]
pub struct AuthCheck {
    allow_token: bool,
    endpoint_scope: Option<EndpointScope>,
    crate_name: Option<String>,
}

impl AuthCheck {
    #[must_use]
    // #[must_use] can't be applied in the `Default` trait impl
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self {
            allow_token: true,
            endpoint_scope: None,
            crate_name: None,
        }
    }

    #[must_use]
    pub fn only_cookie() -> Self {
        Self {
            allow_token: false,
            endpoint_scope: None,
            crate_name: None,
        }
    }

    pub fn with_endpoint_scope(&self, endpoint_scope: EndpointScope) -> Self {
        Self {
            allow_token: self.allow_token,
            endpoint_scope: Some(endpoint_scope),
            crate_name: self.crate_name.clone(),
        }
    }

    pub fn for_crate(&self, crate_name: &str) -> Self {
        Self {
            allow_token: self.allow_token,
            endpoint_scope: self.endpoint_scope,
            crate_name: Some(crate_name.to_string()),
        }
    }

    #[instrument(name = "auth.check", skip_all)]
    pub fn check(&self, parts: &Parts, conn: &mut impl Conn) -> AppResult<Authentication> {
        let auth = authenticate(parts, conn)?;

        if let Some(token) = auth.api_token() {
            if !self.allow_token {
                let error_message =
                    "API Token authentication was explicitly disallowed for this API";
                parts.request_log().add("cause", error_message);

                return Err(forbidden(
                    "this action can only be performed on the crates.io website",
                ));
            }

            if !self.endpoint_scope_matches(token.endpoint_scopes.as_ref()) {
                let error_message = "Endpoint scope mismatch";
                parts.request_log().add("cause", error_message);

                return Err(forbidden(
                    "this token does not have the required permissions to perform this action",
                ));
            }

            if !self.crate_scope_matches(token.crate_scopes.as_ref()) {
                let error_message = "Crate scope mismatch";
                parts.request_log().add("cause", error_message);

                return Err(forbidden(
                    "this token does not have the required permissions to perform this action",
                ));
            }
        }

        Ok(auth)
    }

    fn endpoint_scope_matches(&self, token_scopes: Option<&Vec<EndpointScope>>) -> bool {
        match (&token_scopes, &self.endpoint_scope) {
            // The token is a legacy token.
            (None, _) => true,

            // The token is NOT a legacy token, and the endpoint only allows legacy tokens.
            (Some(_), None) => false,

            // The token is NOT a legacy token, and the endpoint allows a certain endpoint scope or a legacy token.
            (Some(token_scopes), Some(endpoint_scope)) => token_scopes.contains(endpoint_scope),
        }
    }

    fn crate_scope_matches(&self, token_scopes: Option<&Vec<CrateScope>>) -> bool {
        match (&token_scopes, &self.crate_name) {
            // The token is a legacy token.
            (None, _) => true,

            // The token does not have any crate scopes.
            (Some(token_scopes), _) if token_scopes.is_empty() => true,

            // The token has crate scopes, but the endpoint does not deal with crates.
            (Some(_), None) => false,

            // The token is NOT a legacy token, and the endpoint allows a certain endpoint scope or a legacy token.
            (Some(token_scopes), Some(crate_name)) => token_scopes
                .iter()
                .any(|token_scope| token_scope.matches(crate_name)),
        }
    }
}

#[derive(Debug)]
pub enum Authentication {
    Cookie(CookieAuthentication),
    Token(TokenAuthentication),
}

#[derive(Debug)]
pub struct CookieAuthentication {
    user: User,
}

#[derive(Debug)]
pub struct TokenAuthentication {
    token: ApiToken,
    user: User,
}

impl Authentication {
    pub fn user_id(&self) -> i32 {
        self.user().id
    }

    pub fn api_token_id(&self) -> Option<i32> {
        self.api_token().map(|token| token.id)
    }

    pub fn api_token(&self) -> Option<&ApiToken> {
        match self {
            Authentication::Token(token) => Some(&token.token),
            _ => None,
        }
    }

    pub fn user(&self) -> &User {
        match self {
            Authentication::Cookie(cookie) => &cookie.user,
            Authentication::Token(token) => &token.user,
        }
    }
}

#[instrument(skip_all)]
fn authenticate_via_cookie(
    parts: &Parts,
    conn: &mut impl Conn,
) -> AppResult<Option<CookieAuthentication>> {
    let user_id_from_session = parts
        .session()
        .get("user_id")
        .and_then(|s| s.parse::<i32>().ok());

    let Some(id) = user_id_from_session else {
        return Ok(None);
    };

    let user = User::find(conn, id).map_err(|err| {
        parts.request_log().add("cause", err);
        internal("user_id from cookie not found in database")
    })?;

    ensure_not_locked(&user)?;

    parts.request_log().add("uid", id);

    Ok(Some(CookieAuthentication { user }))
}

#[instrument(skip_all)]
fn authenticate_via_token(
    parts: &Parts,
    conn: &mut impl Conn,
) -> AppResult<Option<TokenAuthentication>> {
    let maybe_authorization = parts
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let Some(header_value) = maybe_authorization else {
        return Ok(None);
    };

    let token =
        HashedToken::parse(header_value).map_err(|_| InsecurelyGeneratedTokenRevoked::boxed())?;

    let token = ApiToken::find_by_api_token(conn, &token).map_err(|e| {
        let cause = format!("invalid token caused by {e}");
        parts.request_log().add("cause", cause);

        forbidden("authentication failed")
    })?;

    let user = User::find(conn, token.user_id).map_err(|err| {
        parts.request_log().add("cause", err);
        internal("user_id from token not found in database")
    })?;

    ensure_not_locked(&user)?;

    parts.request_log().add("uid", token.user_id);
    parts.request_log().add("tokenid", token.id);

    Ok(Some(TokenAuthentication { user, token }))
}

#[instrument(skip_all)]
fn authenticate(parts: &Parts, conn: &mut impl Conn) -> AppResult<Authentication> {
    controllers::util::verify_origin(parts)?;

    match authenticate_via_cookie(parts, conn) {
        Ok(None) => {}
        Ok(Some(auth)) => return Ok(Authentication::Cookie(auth)),
        Err(err) => return Err(err),
    }

    match authenticate_via_token(parts, conn) {
        Ok(None) => {}
        Ok(Some(auth)) => return Ok(Authentication::Token(auth)),
        Err(err) => return Err(err),
    }

    // Unable to authenticate the user
    let cause = "no cookie session or auth header found";
    parts.request_log().add("cause", cause);

    return Err(forbidden("this action requires authentication"));
}

fn ensure_not_locked(user: &User) -> AppResult<()> {
    if let Some(reason) = &user.account_lock_reason {
        let still_locked = user
            .account_lock_until
            .map(|until| until > Utc::now().naive_utc())
            .unwrap_or(true);

        if still_locked {
            return Err(account_locked(reason, user.account_lock_until));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cs(scope: &str) -> CrateScope {
        CrateScope::try_from(scope).unwrap()
    }

    #[test]
    fn regular_endpoint() {
        let auth_check = AuthCheck::default();

        assert!(auth_check.endpoint_scope_matches(None));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::PublishNew])));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::PublishUpdate])));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::Yank])));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::ChangeOwners])));

        assert!(auth_check.crate_scope_matches(None));
        assert!(!auth_check.crate_scope_matches(Some(&vec![cs("tokio-console")])));
        assert!(!auth_check.crate_scope_matches(Some(&vec![cs("tokio-*")])));
    }

    #[test]
    fn publish_new_endpoint() {
        let auth_check = AuthCheck::default()
            .with_endpoint_scope(EndpointScope::PublishNew)
            .for_crate("tokio-console");

        assert!(auth_check.endpoint_scope_matches(None));
        assert!(auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::PublishNew])));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::PublishUpdate])));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::Yank])));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::ChangeOwners])));

        assert!(auth_check.crate_scope_matches(None));
        assert!(auth_check.crate_scope_matches(Some(&vec![cs("tokio-console")])));
        assert!(auth_check.crate_scope_matches(Some(&vec![cs("tokio-*")])));
        assert!(!auth_check.crate_scope_matches(Some(&vec![cs("anyhow")])));
        assert!(!auth_check.crate_scope_matches(Some(&vec![cs("actix-*")])));
    }

    #[test]
    fn publish_update_endpoint() {
        let auth_check = AuthCheck::default()
            .with_endpoint_scope(EndpointScope::PublishUpdate)
            .for_crate("tokio-console");

        assert!(auth_check.endpoint_scope_matches(None));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::PublishNew])));
        assert!(auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::PublishUpdate])));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::Yank])));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::ChangeOwners])));

        assert!(auth_check.crate_scope_matches(None));
        assert!(auth_check.crate_scope_matches(Some(&vec![cs("tokio-console")])));
        assert!(auth_check.crate_scope_matches(Some(&vec![cs("tokio-*")])));
        assert!(!auth_check.crate_scope_matches(Some(&vec![cs("anyhow")])));
        assert!(!auth_check.crate_scope_matches(Some(&vec![cs("actix-*")])));
    }

    #[test]
    fn yank_endpoint() {
        let auth_check = AuthCheck::default()
            .with_endpoint_scope(EndpointScope::Yank)
            .for_crate("tokio-console");

        assert!(auth_check.endpoint_scope_matches(None));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::PublishNew])));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::PublishUpdate])));
        assert!(auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::Yank])));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::ChangeOwners])));

        assert!(auth_check.crate_scope_matches(None));
        assert!(auth_check.crate_scope_matches(Some(&vec![cs("tokio-console")])));
        assert!(auth_check.crate_scope_matches(Some(&vec![cs("tokio-*")])));
        assert!(!auth_check.crate_scope_matches(Some(&vec![cs("anyhow")])));
        assert!(!auth_check.crate_scope_matches(Some(&vec![cs("actix-*")])));
    }

    #[test]
    fn owner_change_endpoint() {
        let auth_check = AuthCheck::default()
            .with_endpoint_scope(EndpointScope::ChangeOwners)
            .for_crate("tokio-console");

        assert!(auth_check.endpoint_scope_matches(None));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::PublishNew])));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::PublishUpdate])));
        assert!(!auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::Yank])));
        assert!(auth_check.endpoint_scope_matches(Some(&vec![EndpointScope::ChangeOwners])));

        assert!(auth_check.crate_scope_matches(None));
        assert!(auth_check.crate_scope_matches(Some(&vec![cs("tokio-console")])));
        assert!(auth_check.crate_scope_matches(Some(&vec![cs("tokio-*")])));
        assert!(!auth_check.crate_scope_matches(Some(&vec![cs("anyhow")])));
        assert!(!auth_check.crate_scope_matches(Some(&vec![cs("actix-*")])));
    }
}
