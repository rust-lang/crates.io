use crate::auth::Permission;
use crate::controllers::helpers::authorization::Rights;
use crate::middleware::app::RequestApp;
use crate::middleware::log_request::RequestLogExt;
use crate::util::errors::{BoxedAppError, account_locked, bad_request, forbidden};
use chrono::Utc;
use crates_io_database::models::token::EndpointScope;
use crates_io_database::models::{ApiToken, OwnerKind, User};
use crates_io_database::schema::crate_owners;
use diesel::dsl::exists;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use http::request::Parts;

pub struct AuthorizedUser<T> {
    user: User,
    api_token: T,
}

impl<T> AuthorizedUser<T> {
    pub fn new(user: User, api_token: T) -> Self {
        AuthorizedUser { user, api_token }
    }

    pub fn user(&self) -> &User {
        &self.user
    }

    pub fn user_id(&self) -> i32 {
        self.user.id
    }

    fn check_user_locked(&self) -> Result<(), BoxedAppError> {
        ensure_not_locked(&self.user)
    }

    async fn check_email_verification(
        &self,
        conn: &mut AsyncPgConnection,
        permission: &Permission<'_>,
    ) -> Result<(), BoxedAppError> {
        if self.user.verified_email(conn).await?.is_some() {
            return Ok(());
        }

        match permission {
            Permission::PublishNew { .. } | Permission::PublishUpdate { .. } => Err(bad_request(
                "A verified email address is required to publish crates to crates.io. Visit https://crates.io/settings/profile to set and verify your email address.",
            )),
            Permission::CreateTrustPubGitHubConfig { .. } => Err(forbidden(
                "You must verify your email address to create a Trusted Publishing config",
            )),
            _ => Ok(()),
        }
    }

    async fn check_crate_rights(
        &self,
        conn: &mut AsyncPgConnection,
        parts: &Parts,
        permission: &Permission<'_>,
    ) -> Result<(), BoxedAppError> {
        match permission {
            Permission::PublishUpdate { krate } => {
                const MISSING_RIGHTS_ERROR_MESSAGE: &str = "this crate exists but you don't seem to be an owner. \
                    If you believe this is a mistake, perhaps you need \
                    to accept an invitation to be an owner before publishing.";

                let owners = krate.owners(conn).await?;
                if Rights::get(&self.user, &*parts.app().github, &owners).await? < Rights::Publish {
                    return Err(forbidden(MISSING_RIGHTS_ERROR_MESSAGE));
                }
            }
            Permission::DeleteCrate { owners, .. } => {
                match Rights::get(&self.user, &*parts.app().github, owners).await? {
                    Rights::Full => {}
                    Rights::Publish => {
                        return Err(forbidden(
                            "team members don't have permission to delete crates",
                        ));
                    }
                    Rights::None => {
                        return Err(forbidden("only owners have permission to delete crates"));
                    }
                }
            }
            Permission::ModifyOwners { owners, .. } => {
                match Rights::get(&self.user, &*parts.app().github, owners).await? {
                    Rights::Full => {}
                    Rights::Publish => {
                        return Err(forbidden(
                            "team members don't have permission to modify owners",
                        ));
                    }
                    Rights::None => {
                        return Err(forbidden("only owners have permission to modify owners"));
                    }
                }
            }
            Permission::ListTrustPubGitHubConfigs { krate } => {
                let is_owner = diesel::select(exists(
                    crate_owners::table
                        .filter(crate_owners::crate_id.eq(krate.id))
                        .filter(crate_owners::deleted.eq(false))
                        .filter(crate_owners::owner_kind.eq(OwnerKind::User))
                        .filter(crate_owners::owner_id.eq(self.user.id)),
                ))
                .get_result::<bool>(conn)
                .await?;

                if !is_owner {
                    return Err(bad_request("You are not an owner of this crate"));
                }
            }
            Permission::CreateTrustPubGitHubConfig { user_owner_ids } => {
                if user_owner_ids.iter().all(|id| *id != self.user.id) {
                    return Err(bad_request("You are not an owner of this crate"));
                }
            }
            Permission::DeleteTrustPubGitHubConfig { user_owner_ids } => {
                if user_owner_ids.iter().all(|id| *id != self.user.id) {
                    return Err(bad_request("You are not an owner of this crate"));
                }
            }
            Permission::UpdateVersion { krate } => {
                let owners = krate.owners(conn).await?;
                if Rights::get(&self.user, &*parts.app().github, &owners).await? < Rights::Publish {
                    return Err(forbidden("must already be an owner to yank or unyank"));
                }
            }
            Permission::YankVersion { krate } => {
                let owners = krate.owners(conn).await?;
                if Rights::get(&self.user, &*parts.app().github, &owners).await? < Rights::Publish {
                    return Err(forbidden("must already be an owner to yank or unyank"));
                }
            }
            Permission::UnyankVersion { krate } => {
                let owners = krate.owners(conn).await?;
                if Rights::get(&self.user, &*parts.app().github, &owners).await? < Rights::Publish {
                    return Err(forbidden("must already be an owner to yank or unyank"));
                }
            }
            Permission::RebuildDocs { krate } => {
                let owners = krate.owners(conn).await?;
                if Rights::get(&self.user, &*parts.app().github, &owners).await? < Rights::Publish {
                    return Err(forbidden(
                        "user doesn't have permission to trigger a docs rebuild",
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }
}

impl AuthorizedUser<()> {
    pub(in crate::auth) async fn validate(
        self,
        conn: &mut AsyncPgConnection,
        parts: &Parts,
        permission: Permission<'_>,
    ) -> Result<Self, BoxedAppError> {
        if self.user.is_admin && permission.allowed_for_admin() {
            return Ok(self);
        }

        self.check_user_locked()?;
        self.check_email_verification(conn, &permission).await?;
        self.check_crate_rights(conn, parts, &permission).await?;

        Ok(self)
    }
}

impl AuthorizedUser<ApiToken> {
    pub fn api_token(&self) -> &ApiToken {
        &self.api_token
    }

    pub fn api_token_id(&self) -> i32 {
        self.api_token.id
    }

    pub(in crate::auth) async fn validate(
        self,
        conn: &mut AsyncPgConnection,
        parts: &Parts,
        permission: Permission<'_>,
    ) -> Result<Self, BoxedAppError> {
        if self.user.is_admin && permission.allowed_for_admin() {
            return Ok(self);
        }

        self.check_user_locked()?;
        self.check_email_verification(conn, &permission).await?;
        self.check_crate_rights(conn, parts, &permission).await?;
        self.check_token_scopes(parts, &permission).await?;

        Ok(self)
    }

    async fn check_token_scopes(
        &self,
        parts: &Parts,
        permission: &Permission<'_>,
    ) -> Result<(), BoxedAppError> {
        let (endpoint_scope, crate_name) = match permission {
            Permission::PublishNew { name } => (Some(EndpointScope::PublishNew), Some(*name)),
            Permission::PublishUpdate { krate } => (
                Some(EndpointScope::PublishUpdate),
                Some(krate.name.as_str()),
            ),
            Permission::ModifyOwners { krate, .. } => {
                (Some(EndpointScope::ChangeOwners), Some(krate.name.as_str()))
            }
            Permission::UpdateVersion { krate } => {
                (Some(EndpointScope::Yank), Some(krate.name.as_str()))
            }
            Permission::YankVersion { krate } => {
                (Some(EndpointScope::Yank), Some(krate.name.as_str()))
            }
            Permission::UnyankVersion { krate } => {
                (Some(EndpointScope::Yank), Some(krate.name.as_str()))
            }
            _ => (None, None),
        };

        if !endpoint_scope_matches(endpoint_scope, &self.api_token) {
            let error_message = "Endpoint scope mismatch";
            parts.request_log().add("cause", error_message);

            return Err(forbidden(
                "this token does not have the required permissions to perform this action",
            ));
        }

        if !crate_scope_matches(crate_name, &self.api_token) {
            let error_message = "Crate scope mismatch";
            parts.request_log().add("cause", error_message);

            return Err(forbidden(
                "this token does not have the required permissions to perform this action",
            ));
        }

        Ok(())
    }
}

impl AuthorizedUser<Option<ApiToken>> {
    pub fn api_token(&self) -> Option<&ApiToken> {
        self.api_token.as_ref()
    }

    pub fn api_token_id(&self) -> Option<i32> {
        self.api_token.as_ref().map(|token| token.id)
    }
}

impl From<AuthorizedUser<()>> for AuthorizedUser<Option<ApiToken>> {
    fn from(auth: AuthorizedUser<()>) -> Self {
        AuthorizedUser {
            user: auth.user,
            api_token: None,
        }
    }
}

impl From<AuthorizedUser<ApiToken>> for AuthorizedUser<Option<ApiToken>> {
    fn from(auth: AuthorizedUser<ApiToken>) -> Self {
        AuthorizedUser {
            user: auth.user,
            api_token: Some(auth.api_token),
        }
    }
}

fn ensure_not_locked(user: &User) -> Result<(), BoxedAppError> {
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

fn endpoint_scope_matches(endpoint_scope: Option<EndpointScope>, token: &ApiToken) -> bool {
    match (&token.endpoint_scopes, endpoint_scope) {
        // The token is a legacy token.
        (None, _) => true,

        // The token is NOT a legacy token, and the endpoint only allows legacy tokens.
        (Some(_), None) => false,

        // The token is NOT a legacy token, and the endpoint allows a certain endpoint scope or a legacy token.
        (Some(token_scopes), Some(endpoint_scope)) => token_scopes.contains(&endpoint_scope),
    }
}

fn crate_scope_matches(crate_name: Option<&str>, token: &ApiToken) -> bool {
    match (&token.crate_scopes, &crate_name) {
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
