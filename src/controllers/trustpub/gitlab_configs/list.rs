use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::pagination::{
    Page, PaginationOptions, PaginationQueryParams, encode_seek,
};
use crate::controllers::krate::load_crate;
use crate::controllers::trustpub::gitlab_configs::json::{self, ListResponse, ListResponseMeta};
use crate::util::RequestUtils;
use crate::util::errors::{AppResult, bad_request};
use axum::Json;
use axum::extract::{FromRequestParts, Query};
use crates_io_database::models::OwnerKind;
use crates_io_database::models::token::EndpointScope;
use crates_io_database::models::trustpub::GitLabConfig;
use crates_io_database::schema::{crate_owners, trustpub_configs_gitlab};
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
    pub krate: String,
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
    let mut conn = state.db_read().await?;

    let auth = AuthCheck::default()
        .with_endpoint_scope(EndpointScope::TrustedPublishing)
        .for_crate(&params.krate)
        .check(&parts, &mut conn)
        .await?;
    let auth_user = auth.user();

    let krate = load_crate(&mut conn, &params.krate).await?;

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

    let pagination = PaginationOptions::builder()
        .enable_seek(true)
        .enable_pages(false)
        .gather(&parts)?;

    let (configs, total, next_page) =
        list_configs(&mut conn, krate.id, &pagination, &parts).await?;

    let gitlab_configs = configs
        .into_iter()
        .map(|config| json::GitLabConfig {
            id: config.id,
            krate: krate.name.clone(),
            namespace: config.namespace,
            namespace_id: config.namespace_id,
            project: config.project,
            workflow_filepath: config.workflow_filepath,
            environment: config.environment,
            created_at: config.created_at,
        })
        .collect();

    Ok(Json(ListResponse {
        gitlab_configs,
        meta: ListResponseMeta { total, next_page },
    }))
}

async fn list_configs(
    conn: &mut diesel_async::AsyncPgConnection,
    crate_id: i32,
    options: &PaginationOptions,
    req: &Parts,
) -> AppResult<(Vec<GitLabConfig>, i64, Option<String>)> {
    use seek::*;

    let seek = Seek::Id;

    assert!(
        !matches!(&options.page, Page::Numeric(_)),
        "?page= is not supported"
    );

    let make_base_query = || {
        GitLabConfig::query()
            .filter(trustpub_configs_gitlab::crate_id.eq(crate_id))
            .into_boxed()
    };

    let mut query = make_base_query();
    query = query.limit(options.per_page);
    query = query.order(trustpub_configs_gitlab::id.asc());

    if let Some(SeekPayload::Id(Id { id })) = seek.after(&options.page)? {
        query = query.filter(trustpub_configs_gitlab::id.gt(id));
    }

    let data: Vec<GitLabConfig> = query.load(conn).await?;

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
    use crate::controllers::helpers::pagination::seek;
    use crates_io_database::models::trustpub::GitLabConfig;

    seek!(
        pub enum Seek {
            Id { id: i32 },
        }
    );

    impl Seek {
        pub(crate) fn to_payload(&self, record: &GitLabConfig) -> SeekPayload {
            match *self {
                Seek::Id => SeekPayload::Id(Id { id: record.id }),
            }
        }
    }
}
