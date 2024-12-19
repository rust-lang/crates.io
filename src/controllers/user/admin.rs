use axum::{extract::Path, Json};
use crates_io_database::schema::{emails, users};
use diesel::{pg::Pg, prelude::*};
use diesel_async::{AsyncConnection, RunQueryDsl};
use http::request::Parts;

use crate::{
    app::AppState, auth::AuthCheck, models::User, sql::lower, util::errors::AppResult,
    views::EncodableAdminUser,
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
