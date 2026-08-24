use crate::app::AppState;
use crate::models::{CrateOwner, OwnerKind, PublicUser, users_by_username};
use crate::schema::{crate_downloads, crate_owners, crates, oauth_github};
use crate::util::errors::{AppResult, BoxedAppError, bad_request};
use crate::views::{EncodableLinkedAccount, EncodablePublicUser};
use axum::Json;
use axum::extract::{FromRequestParts, Path};
use axum_extra::extract::Query;
use bigdecimal::{BigDecimal, ToPrimitive};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Query parameters for finding a user.
#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct UserQueryParams {
    /// Additional data to include in the response.
    ///
    /// Valid values: `linked_accounts`.
    ///
    /// Defaults to no additional data.
    ///
    /// This parameter expects a comma-separated list of values.
    include: Option<String>,
}

/// Response returned when getting a user by login.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UserGetResponse {
    pub user: EncodablePublicUser,

    /// Public linked accounts, if the client requested them.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub linked_accounts: Option<Vec<EncodableLinkedAccount>>,
}

/// Find user by login.
#[utoipa::path(
    get,
    path = "/api/v1/users/{user}",
    params(
        ("user" = String, Path, description = "crates.io username"),
        UserQueryParams,
    ),
    tag = "users",
    responses(
        (status = 200, description = "Successful Response", body = inline(UserGetResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn find_user(
    state: AppState,
    Path(user_name): Path<String>,
    params: UserQueryParams,
) -> AppResult<Json<UserGetResponse>> {
    let mut conn = state.db_read_prefer_primary().await?;
    let include = params
        .include
        .as_deref()
        .map(ShowIncludeMode::from_str)
        .transpose()?
        .unwrap_or_default();

    let user = users_by_username(&user_name)
        .left_join(oauth_github::table)
        .select(PublicUser::as_select())
        .first(&mut conn)
        .await?;

    let linked_accounts = if include.linked_accounts {
        let accounts = oauth_github::table
            .filter(oauth_github::user_id.eq(user.id))
            .select((
                oauth_github::account_id,
                oauth_github::login,
                oauth_github::avatar,
            ))
            .order(oauth_github::account_id.asc())
            .load(&mut conn)
            .await?;

        let accounts = accounts
            .into_iter()
            .map(|(account_id, login, avatar)| {
                EncodableLinkedAccount::github(account_id, login, avatar)
            })
            .collect();
        Some(accounts)
    } else {
        None
    };
    Ok(Json(UserGetResponse {
        user: user.into(),
        linked_accounts,
    }))
}

#[derive(Debug, Default)]
struct ShowIncludeMode {
    linked_accounts: bool,
}

impl FromStr for ShowIncludeMode {
    type Err = BoxedAppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        const INVALID_COMPONENT: &str =
            "invalid component for ?include= (expected 'linked_accounts')";

        let mut mode = Self::default();
        for component in value.split(',') {
            match component {
                "" => {}
                "linked_accounts" => mode.linked_accounts = true,
                _ => return Err(bad_request(INVALID_COMPONENT)),
            }
        }
        Ok(mode)
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct StatsResponse {
    /// The total number of downloads for crates owned by the user.
    #[schema(example = 123_456_789)]
    pub total_downloads: u64,
}

/// Get user stats.
///
/// This currently only returns the total number of downloads for crates owned
/// by the user.
#[utoipa::path(
    get,
    path = "/api/v1/users/{id}/stats",
    params(
        ("id" = i32, Path, description = "ID of the user"),
    ),
    tag = "users",
    responses(
        (status = 200, description = "Successful Response", body = inline(StatsResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn get_user_stats(
    state: AppState,
    Path(user_id): Path<i32>,
) -> AppResult<Json<StatsResponse>> {
    let mut conn = state.db_read_prefer_primary().await?;

    use diesel::dsl::sum;
    use diesel_async::RunQueryDsl;

    let total_downloads = CrateOwner::by_owner_kind(OwnerKind::User)
        .inner_join(crates::table)
        .inner_join(crate_downloads::table.on(crates::id.eq(crate_downloads::crate_id)))
        .filter(crate_owners::owner_id.eq(user_id))
        .select(sum(crate_downloads::downloads))
        .first::<Option<BigDecimal>>(&mut conn)
        .await?
        .map(|d| d.to_u64().unwrap_or(u64::MAX))
        .unwrap_or(0);

    Ok(Json(StatsResponse { total_downloads }))
}
