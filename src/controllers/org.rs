use crate::models::Team;
use crate::server::ServerContext;
use crate::util::errors::{AppResult, bad_request};
use crate::views::EncodableTeam;
use axum::Json;
use axum::extract::Path;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Serialize;

/// Response returned when getting teams by org.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OrgTeamsGetResponse {
    teams: Vec<EncodableTeam>,
}

/// Find teams by organization that crates.io knows about.
///
/// Specify the organization with `service:org_name`. For example, to list all GitHub teams in the
/// `rust-lang` organization, specify `github:rust-lang`. If there are no teams for the specified
/// organization, this returns an empty list.
#[utoipa::path(
    get,
    path = "/api/v1/org/{org}",
    params(
        ("org" = String, Path, description = "Name of the organization", example = "github:rust-lang"),
    ),
    tag = "org",
    responses(
        (status = 200, description = "Successful Response", body = inline(OrgTeamsGetResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn find_teams(
    ctx: ServerContext,
    Path(name): Path<String>,
) -> AppResult<Json<OrgTeamsGetResponse>> {
    use crate::schema::teams::dsl::login;
    let mut conn = ctx.db_read().await?;

    let name = name.to_lowercase();

    // Validate `name` format; return error if invalid
    let mut chunks = name.split(':');
    let first = chunks.next().unwrap_or_default();
    let parts = (first, chunks.next(), chunks.next());
    match parts {
        (_, _, Some(_)) => {
            return Err(bad_request(
                "organizations must have at most two components",
            ));
        }
        ("", _, _) | (_, Some(""), _) => {
            return Err(bad_request("organization components must not be empty"));
        }
        (_, None, _) => {
            return Err(bad_request(
                "organizations must have at least two components",
            ));
        }
        (_, Some(_), None) => (),
    }

    let teams: Vec<Team> = Team::query()
        .filter(login.like(format!("{name}:%")))
        .get_results(&mut conn)
        .await?;

    let teams = teams.into_iter().map(EncodableTeam::from).collect();
    Ok(Json(OrgTeamsGetResponse { teams }))
}
