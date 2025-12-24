use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::pagination::{
    Page, PaginationOptions, PaginationQueryParams, encode_seek,
};
use crate::controllers::krate::load_crate;
use crate::controllers::trustpub::gitlab_configs::json::{self, ListResponse, ListResponseMeta};
use crate::util::RequestUtils;
use crate::util::errors::{AppResult, bad_request, forbidden};
use axum::Json;
use axum::extract::{FromRequestParts, Query};
use crates_io_database::models::OwnerKind;
use crates_io_database::models::token::EndpointScope;
use crates_io_database::models::trustpub::GitLabConfig;
use crates_io_database::schema::{crate_owners, crates, trustpub_configs_gitlab};
use diesel::dsl::{exists, select};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::request::Parts;
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct ListQueryParams {
    /// Name of the crate to list Trusted Publishing configurations for.
    #[serde(rename = "crate")]
    pub krate: Option<String>,

    /// User ID to list Trusted Publishing configurations for all crates owned by the user.
    pub user_id: Option<i32>,
}

/// List Trusted Publishing configurations for GitLab CI/CD.
#[utoipa::path(
    get,
    path = "/api/v1/trusted_publishing/gitlab_configs",
    params(ListQueryParams, PaginationQueryParams),
    security(("cookie" = []), ("api_token" = [])),
    tag = "trusted_publishing",
    responses((status = 200, description = "Successful Response", body = inline(ListResponse))),
)]
pub async fn list_trustpub_gitlab_configs(
    state: AppState,
    params: ListQueryParams,
    parts: Parts,
) -> AppResult<Json<ListResponse>> {
    match (&params.krate, params.user_id) {
        (Some(krate), None) => list_by_crate(state, krate, parts).await,
        (None, Some(user_id)) => list_by_user(state, user_id, parts).await,
        (Some(_), Some(_)) => Err(bad_request(
            "Cannot specify both `crate` and `user_id` query parameters",
        )),
        (None, None) => Err(bad_request(
            "Must specify either `crate` or `user_id` query parameter",
        )),
    }
}

async fn list_by_crate(
    state: AppState,
    krate_name: &str,
    parts: Parts,
) -> AppResult<Json<ListResponse>> {
    let mut conn = state.db_read().await?;

    let auth = AuthCheck::default()
        .with_endpoint_scope(EndpointScope::TrustedPublishing)
        .for_crate(krate_name)
        .check(&parts, &mut conn)
        .await?;
    let auth_user = auth.user();

    let krate = load_crate(&mut conn, krate_name).await?;

    // Check if the authenticated user is an owner of the crate
    let is_owner = select(exists(
        crate_owners::table
            .filter(crate_owners::crate_id.eq(krate.id))
            .filter(crate_owners::deleted.eq(false))
            .filter(crate_owners::owner_kind.eq(OwnerKind::User))
            .filter(crate_owners::owner_id.eq(auth_user.id)),
    ))
    .get_result::<bool>(&mut conn)
    .await?;

    if !is_owner {
        return Err(bad_request("You are not an owner of this crate"));
    }

    paginated_response(&mut conn, &[krate.id], &parts).await
}

async fn list_by_user(
    state: AppState,
    user_id: i32,
    parts: Parts,
) -> AppResult<Json<ListResponse>> {
    let mut conn = state.db_read().await?;

    let auth = AuthCheck::default()
        .with_endpoint_scope(EndpointScope::TrustedPublishing)
        .allow_any_crate_scope()
        .check(&parts, &mut conn)
        .await?;

    // Reject legacy tokens for this endpoint
    auth.reject_legacy_tokens()?;

    let auth_user = auth.user();

    // Verify the authenticated user matches the requested user_id
    if auth_user.id != user_id {
        return Err(forbidden(
            "this action requires authentication as the specified user",
        ));
    }

    // Get crate scopes from the token (if any)
    let crate_scopes = auth.api_token().and_then(|t| t.crate_scopes.as_ref());

    // Get all crate IDs owned by the user
    let mut owned_crates: Vec<(i32, String)> = crate_owners::table
        .inner_join(crates::table)
        .filter(crate_owners::owner_id.eq(user_id))
        .filter(crate_owners::owner_kind.eq(OwnerKind::User))
        .filter(crate_owners::deleted.eq(false))
        .select((crates::id, crates::name))
        .load(&mut conn)
        .await?;

    // Filter by crate scopes if the token has any
    if let Some(scopes) = crate_scopes
        && !scopes.is_empty()
    {
        owned_crates.retain(|(_, name)| scopes.iter().any(|scope| scope.matches(name)));
    }

    let crate_ids: Vec<i32> = owned_crates.iter().map(|(id, _)| *id).collect();

    paginated_response(&mut conn, &crate_ids, &parts).await
}

async fn paginated_response(
    conn: &mut diesel_async::AsyncPgConnection,
    crate_ids: &[i32],
    parts: &Parts,
) -> AppResult<Json<ListResponse>> {
    let pagination = PaginationOptions::builder()
        .enable_seek(true)
        .enable_pages(false)
        .gather(parts)?;

    let (configs, total, next_page) = list_configs(conn, crate_ids, &pagination, parts).await?;

    let gitlab_configs = configs.into_iter().map(to_json_config).collect();

    Ok(Json(ListResponse {
        gitlab_configs,
        meta: ListResponseMeta { total, next_page },
    }))
}

fn to_json_config(config: ConfigWithCrateName) -> json::GitLabConfig {
    let crate_name = config.crate_name;
    let config = config.config;

    json::GitLabConfig {
        id: config.id,
        krate: crate_name,
        namespace: config.namespace,
        namespace_id: config.namespace_id,
        project: config.project,
        workflow_filepath: config.workflow_filepath,
        environment: config.environment,
        created_at: config.created_at,
    }
}

#[derive(Debug, HasQuery)]
#[diesel(base_query = trustpub_configs_gitlab::table.inner_join(crates::table))]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct ConfigWithCrateName {
    #[diesel(select_expression = crates::name)]
    crate_name: String,
    #[diesel(embed)]
    config: GitLabConfig,
}

async fn list_configs(
    conn: &mut diesel_async::AsyncPgConnection,
    crate_ids: &[i32],
    options: &PaginationOptions,
    req: &Parts,
) -> AppResult<(Vec<ConfigWithCrateName>, i64, Option<String>)> {
    use seek::*;

    let seek = Seek::Id;

    assert!(
        !matches!(&options.page, Page::Numeric(_)),
        "?page= is not supported"
    );

    let make_base_query = || {
        ConfigWithCrateName::query()
            .filter(trustpub_configs_gitlab::crate_id.eq_any(crate_ids))
            .into_boxed()
    };

    let mut query = make_base_query();
    query = query.limit(options.per_page);
    query = query.order(trustpub_configs_gitlab::id.asc());

    if let Some(SeekPayload::Id(Id { id })) = seek.after(&options.page)? {
        query = query.filter(trustpub_configs_gitlab::id.gt(id));
    }

    let data = query.load(conn).await?;

    let next_page = next_seek_params(&data, options, |last| seek.to_payload(last))?
        .map(|p| req.query_with_params(p));

    // Avoid the count query if we're on the first page and got fewer results than requested
    let total =
        if matches!(options.page, Page::Unspecified) && data.len() < options.per_page as usize {
            data.len() as i64
        } else {
            make_base_query().count().get_result(conn).await?
        };

    Ok((data, total, next_page))
}

fn next_seek_params<T, S, F>(
    records: &[T],
    options: &PaginationOptions,
    f: F,
) -> AppResult<Option<IndexMap<String, String>>>
where
    F: Fn(&T) -> S,
    S: serde::Serialize,
{
    if records.len() < options.per_page as usize {
        return Ok(None);
    }

    let seek = f(records.last().unwrap());
    let mut opts = IndexMap::new();
    opts.insert("seek".into(), encode_seek(seek)?);
    Ok(Some(opts))
}

mod seek {
    use super::ConfigWithCrateName;
    use crate::controllers::helpers::pagination::seek;

    seek!(
        pub enum Seek {
            Id { id: i32 },
        }
    );

    impl Seek {
        pub(crate) fn to_payload(&self, record: &ConfigWithCrateName) -> SeekPayload {
            match *self {
                Seek::Id => SeekPayload::Id(Id {
                    id: record.config.id,
                }),
            }
        }
    }
}
