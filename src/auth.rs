use crate::controllers;
use crate::controllers::util::RequestPartsExt;
use crate::middleware::log_request::RequestLogExt;
use crate::models::token::{CrateScope, EndpointScope};
use crate::models::{ApiToken, User};
use crate::util::errors::{
    AppResult, BoxedAppError, InsecurelyGeneratedTokenRevoked, account_locked, custom, forbidden,
    internal,
};
use crate::util::token::HashedToken;
use axum::extract::FromRequestParts;
use chrono::Utc;
use crates_io_session::SessionExtension;
use diesel_async::AsyncPgConnection;
use http::request::Parts;
use http::{StatusCode, header};
use secrecy::{ExposeSecret, SecretString};
use tracing::instrument;

pub struct AuthHeader(SecretString);

impl AuthHeader {
    pub async fn optional_from_request_parts(parts: &Parts) -> Result<Option<Self>, BoxedAppError> {
        let Some(auth_header) = parts.headers.get(header::AUTHORIZATION) else {
            return Ok(None);
        };

        let auth_header = auth_header.to_str().map_err(|_| {
            let message = "Invalid `Authorization` header: Found unexpected non-ASCII characters";
            custom(StatusCode::UNAUTHORIZED, message)
        })?;

        let (scheme, token) = auth_header.split_once(' ').unwrap_or(("", auth_header));
        if !(scheme.eq_ignore_ascii_case("Bearer") || scheme.is_empty()) {
            let message = format!(
                "Invalid `Authorization` header: Found unexpected authentication scheme `{scheme}`"
            );
            return Err(custom(StatusCode::UNAUTHORIZED, message));
        }

        let token = SecretString::from(token.trim_ascii());
        Ok(Some(AuthHeader(token)))
    }

    pub async fn from_request_parts(parts: &Parts) -> Result<Self, BoxedAppError> {
        let auth = Self::optional_from_request_parts(parts).await?;
        auth.ok_or_else(|| {
            let message = "Missing `Authorization` header";
            custom(StatusCode::UNAUTHORIZED, message)
        })
    }

    pub fn token(&self) -> &SecretString {
        &self.0
    }
}

impl<S: Send + Sync> FromRequestParts<S> for AuthHeader {
    type Rejection = BoxedAppError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Self::from_request_parts(parts).await
    }
}

#[derive(Debug, Clone)]
pub struct AuthCheck {
    allow_token: bool,
    endpoint_scope: Option<EndpointScope>,
    crate_name: Option<String>,
    allow_any_crate_scope: bool,
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
            allow_any_crate_scope: false,
        }
    }

    #[must_use]
    pub fn only_cookie() -> Self {
        Self {
            allow_token: false,
            endpoint_scope: None,
            crate_name: None,
            allow_any_crate_scope: false,
        }
    }

    pub fn with_endpoint_scope(&self, endpoint_scope: EndpointScope) -> Self {
        Self {
            allow_token: self.allow_token,
            endpoint_scope: Some(endpoint_scope),
            crate_name: self.crate_name.clone(),
            allow_any_crate_scope: self.allow_any_crate_scope,
        }
    }

    pub fn for_crate(&self, crate_name: &str) -> Self {
        Self {
            allow_token: self.allow_token,
            endpoint_scope: self.endpoint_scope,
            crate_name: Some(crate_name.to_string()),
            allow_any_crate_scope: self.allow_any_crate_scope,
        }
    }

    /// Allow tokens with any crate scope without specifying a particular crate.
    ///
    /// Use this for endpoints that deal with multiple crates at once, where the
    /// caller will handle crate scope filtering manually.
    pub fn allow_any_crate_scope(&self) -> Self {
        Self {
            allow_token: self.allow_token,
            endpoint_scope: self.endpoint_scope,
            crate_name: self.crate_name.clone(),
            allow_any_crate_scope: true,
        }
    }

    #[instrument(name = "auth.check", skip_all)]
    pub async fn check(
        &self,
        parts: &Parts,
        conn: &mut AsyncPgConnection,
    ) -> AppResult<Authentication> {
        let auth = authenticate(parts, conn).await?;

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
            // However, if allow_any_crate_scope is set, we allow it (caller handles filtering).
            (Some(_), None) => self.allow_any_crate_scope,

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

    /// Returns an error if the request was authenticated with a legacy API token.
    ///
    /// Legacy tokens are tokens without any endpoint scopes. They were created
    /// before the scoped token feature was introduced.
    pub fn reject_legacy_tokens(&self) -> AppResult<()> {
        if let Some(token) = self.api_token()
            && token.endpoint_scopes.is_none()
        {
            return Err(forbidden(
                "This endpoint cannot be used with legacy API tokens. Use a scoped API token instead.",
            ));
        }
        Ok(())
    }
}

#[instrument(skip_all)]
async fn authenticate_via_cookie(
    parts: &Parts,
    conn: &mut AsyncPgConnection,
) -> AppResult<Option<CookieAuthentication>> {
    let session = parts
        .extensions()
        .get::<SessionExtension>()
        .expect("missing cookie session");

    let user_id_from_session = session.get("user_id").and_then(|s| s.parse::<i32>().ok());
    let Some(id) = user_id_from_session else {
        return Ok(None);
    };

    let user = User::find(conn, id).await.map_err(|err| {
        parts.request_log().add("cause", err);
        internal("user_id from cookie not found in database")
    })?;

    ensure_not_locked(&user)?;

    parts.request_log().add("uid", id);

    Ok(Some(CookieAuthentication { user }))
}

#[instrument(skip_all)]
async fn authenticate_via_token(
    parts: &Parts,
    conn: &mut AsyncPgConnection,
) -> AppResult<Option<TokenAuthentication>> {
    let Some(auth_header) = AuthHeader::optional_from_request_parts(parts).await? else {
        return Ok(None);
    };

    let token = auth_header.token().expose_secret();
    let token = HashedToken::parse(token).map_err(|_| InsecurelyGeneratedTokenRevoked::boxed())?;

    let token = ApiToken::find_by_api_token(conn, &token)
        .await
        .map_err(|e| {
            let cause = format!("invalid token caused by {e}");
            parts.request_log().add("cause", cause);

            forbidden("authentication failed")
        })?;

    let user = User::find(conn, token.user_id).await.map_err(|err| {
        parts.request_log().add("cause", err);
        internal("user_id from token not found in database")
    })?;

    ensure_not_locked(&user)?;

    parts.request_log().add("uid", token.user_id);
    parts.request_log().add("tokenid", token.id);

    Ok(Some(TokenAuthentication { user, token }))
}

#[instrument(skip_all)]
async fn authenticate(parts: &Parts, conn: &mut AsyncPgConnection) -> AppResult<Authentication> {
    controllers::util::verify_origin(parts)?;

    match authenticate_via_cookie(parts, conn).await {
        Ok(None) => {}
        Ok(Some(auth)) => {
            parts.request_log().add("auth_type", "cookie");
            return Ok(Authentication::Cookie(auth));
        }
        Err(err) => return Err(err),
    }

    match authenticate_via_token(parts, conn).await {
        Ok(None) => {}
        Ok(Some(auth)) => {
            parts.request_log().add("auth_type", "token");
            return Ok(Authentication::Token(auth));
        }
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
            .map(|until| until > Utc::now())
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
