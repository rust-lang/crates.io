use axum::{Json, extract::Path};
use axum_extra::{TypedHeader, headers::CacheControl};
use crates_io_api_types::EncodableUserLock;
use crates_io_database::{
    models::{User, users_by_username},
    schema::oauth_github,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::request::Parts;
use serde::{Deserialize, Serialize};

use crate::{
    ServerContext,
    auth::AuthCheck,
    util::{errors::AppResult, no_store},
};

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UserLockGetResponse {
    pub lock: Option<EncodableUserLock>,
}

/// Get the lock status of the given user.
#[utoipa::path(
    get,
    path = "/api/v1/users/{user}/lock",
    params(
        ("user" = String, Path, description = "crates.io username"),
    ),
    security(("cookie" = [])),
    tags = ["users", "admin"],
    extensions(("x-internal" = json!(true))),
    responses(
        (status = 200, description = "Successful Response", body = inline(UserLockGetResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn get(
    ctx: ServerContext,
    Path(user_name): Path<String>,
    req: Parts,
) -> AppResult<(TypedHeader<CacheControl>, Json<UserLockGetResponse>)> {
    let mut conn = ctx.db_read_prefer_primary().await?;

    AuthCheck::only_cookie()
        .require_admin()
        .check(&req, &mut conn)
        .await?;

    let user = users_by_username(&user_name)
        .left_join(oauth_github::table)
        .select(User::as_select())
        .first(&mut conn)
        .await?;

    Ok((
        no_store(),
        Json(UserLockGetResponse {
            lock: user.is_locked().map(|(reason, until)| EncodableUserLock {
                reason: reason.into(),
                until,
            }),
        }),
    ))
}
