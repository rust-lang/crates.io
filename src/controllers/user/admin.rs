use axum::{extract::Path, Json};
use chrono::NaiveDateTime;
use crates_io_database::schema::{emails, users};
use diesel::{pg::Pg, prelude::*};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};
use http::request::Parts;
use utoipa::ToSchema;

use crate::{
    app::AppState, auth::AuthCheck, models::User, sql::lower, util::errors::AppResult,
    util::rfc3339, views::EncodableAdminUser,
};

/// Find user by login, returning the admin's view of the user.
///
/// Only site admins can use this endpoint.
#[utoipa::path(
    get,
    path = "/api/v1/users/{user}/admin",
    params(
        ("user" = String, Path, description = "Login name of the user"),
    ),
    tags = ["admin", "users"],
    responses((status = 200, description = "Successful Response")),
)]
pub async fn get(
    state: AppState,
    Path(user_name): Path<String>,
    req: Parts,
) -> AppResult<Json<EncodableAdminUser>> {
    let mut conn = state.db_read_prefer_primary().await?;
    AuthCheck::only_cookie()
        .require_admin()
        .check(&req, &mut conn)
        .await?;

    get_user(
        |query| query.filter(lower(users::gh_login).eq(lower(user_name))),
        &mut conn,
    )
    .await
    .map(Json)
}

#[derive(Deserialize, ToSchema)]
pub struct LockRequest {
    /// The reason for locking the account. This is visible to the user.
    reason: String,

    /// When to lock the account until. If omitted, the lock will be indefinite.
    #[serde(default, with = "rfc3339::option")]
    until: Option<NaiveDateTime>,
}

/// Lock the given user.
///
/// Only site admins can use this endpoint.
#[utoipa::path(
    put,
    path = "/api/v1/users/{user}/lock",
    params(
        ("user" = String, Path, description = "Login name of the user"),
    ),
    request_body = LockRequest,
    tags = ["admin", "users"],
    responses((status = 200, description = "Successful Response")),
)]
pub async fn lock(
    state: AppState,
    Path(user_name): Path<String>,
    req: Parts,
    Json(LockRequest { reason, until }): Json<LockRequest>,
) -> AppResult<Json<EncodableAdminUser>> {
    let mut conn = state.db_read_prefer_primary().await?;
    AuthCheck::only_cookie()
        .require_admin()
        .check(&req, &mut conn)
        .await?;

    // In theory, we could cook up a complicated update query that returns
    // everything we need to build an `EncodableAdminUser`, but that feels hard.
    // Instead, let's use a small transaction to get the same effect.
    let user = conn
        .transaction(|conn| {
            async move {
                let id = diesel::update(users::table)
                    .filter(lower(users::gh_login).eq(lower(user_name)))
                    .set((
                        users::account_lock_reason.eq(reason),
                        users::account_lock_until.eq(until),
                    ))
                    .returning(users::id)
                    .get_result::<i32>(conn)
                    .await?;

                get_user(|query| query.filter(users::id.eq(id)), conn).await
            }
            .scope_boxed()
        })
        .await?;

    Ok(Json(user))
}

/// A helper to get an [`EncodableAdminUser`] based on whatever filter predicate
/// is provided in the callback.
///
/// It would be ill advised to do anything in `filter` other than calling
/// [`QueryDsl::filter`] on the given query, but I'm not the boss of you.
async fn get_user<Conn, F>(filter: F, conn: &mut Conn) -> AppResult<EncodableAdminUser>
where
    Conn: AsyncConnection<Backend = Pg>,
    F: FnOnce(users::BoxedQuery<'_, Pg>) -> users::BoxedQuery<'_, Pg>,
{
    let query = filter(users::table.into_boxed());

    let (user, verified, email, verification_sent): (User, Option<bool>, Option<String>, bool) =
        query
            .left_join(emails::table)
            .select((
                User::as_select(),
                emails::verified.nullable(),
                emails::email.nullable(),
                emails::token_generated_at.nullable().is_not_null(),
            ))
            .first(conn)
            .await?;

    let verified = verified.unwrap_or(false);
    let verification_sent = verified || verification_sent;
    Ok(EncodableAdminUser::from(
        user,
        email,
        verified,
        verification_sent,
    ))
}
