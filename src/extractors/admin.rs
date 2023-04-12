use axum::{
    async_trait,
    extract::{FromRequestParts, State},
};
use http::request::Parts;

use crate::{app::AppState, auth::AuthCheck, models, util::errors::BoxedAppError};

/// An authorisation extractor that requires that the current user be a valid
/// admin.
///
/// If there is no logged in user, or if the user isn't an admin, then a 403
/// Forbidden will be returned without the controller being invoked.
///
/// Note that the ordering of extractors is often important: most notably, this
/// extractor _must_ be used before any extractor that accesses the `Request` or
/// `RequestParts`.
#[derive(Debug)]
pub struct AdminUser(pub models::user::AdminUser);

#[async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = BoxedAppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let app = State::<AppState>::from_request_parts(parts, state)
            .await
            .expect("accessing AppState");
        let conn = &mut *app.db_read_prefer_primary()?;

        // TODO: allow other authentication methods, such as tokens.
        // TODO: allow token scopes to be required.
        Ok(Self(
            AuthCheck::only_cookie()
                .check(parts, conn)?
                .user()
                .admin()?,
        ))
    }
}
