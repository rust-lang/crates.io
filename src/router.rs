use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use http::{Method, StatusCode};
use utoipa_axum::routes;

use crate::app::AppState;
use crate::controllers::*;
use crate::openapi::BaseOpenApi;
use crate::util::errors::not_found;
use crate::Env;

#[allow(deprecated)]
pub fn build_axum_router(state: AppState) -> Router<()> {
    let (router, openapi) = BaseOpenApi::router()
        // Route used by both `cargo search` and the frontend
        .routes(routes!(krate::search::search))
        // Routes used by `cargo`
        .routes(routes!(krate::publish::publish, krate::metadata::show_new))
        .routes(routes!(
            krate::owners::owners,
            krate::owners::add_owners,
            krate::owners::remove_owners
        ))
        .routes(routes!(version::yank::yank))
        .routes(routes!(version::yank::unyank))
        .routes(routes!(version::downloads::download))
        // Routes used by the frontend
        .routes(routes!(krate::metadata::show, krate::delete::delete))
        .routes(routes!(version::metadata::show, version::metadata::update))
        .routes(routes!(krate::metadata::readme))
        .routes(routes!(version::metadata::dependencies))
        .routes(routes!(version::downloads::downloads))
        .routes(routes!(version::metadata::authors))
        .routes(routes!(krate::downloads::downloads))
        .routes(routes!(krate::versions::versions))
        .routes(routes!(krate::follow::follow, krate::follow::unfollow))
        .routes(routes!(krate::follow::following))
        .routes(routes!(krate::owners::owner_team))
        .routes(routes!(krate::owners::owner_user))
        .routes(routes!(krate::metadata::reverse_dependencies))
        .routes(routes!(keyword::index))
        .routes(routes!(keyword::show))
        .routes(routes!(category::index))
        .routes(routes!(category::show))
        .routes(routes!(category::slugs))
        .routes(routes!(user::other::show, user::update::update_user))
        .routes(routes!(user::other::stats))
        .routes(routes!(team::show_team))
        .split_for_parts();

    let mut router = router
        .route("/api/v1/me", get(user::me::me))
        .route("/api/v1/me/updates", get(user::me::updates))
        .route("/api/v1/me/tokens", get(token::list).put(token::new))
        .route(
            "/api/v1/me/tokens/:id",
            get(token::show).delete(token::revoke),
        )
        .route("/api/v1/tokens/current", delete(token::revoke_current))
        .route(
            "/api/v1/me/crate_owner_invitations",
            get(crate_owner_invitation::list),
        )
        .route(
            "/api/v1/me/crate_owner_invitations/:crate_id",
            put(crate_owner_invitation::handle_invite),
        )
        .route(
            "/api/v1/me/crate_owner_invitations/accept/:token",
            put(crate_owner_invitation::handle_invite_with_token),
        )
        .route(
            "/api/v1/me/email_notifications",
            put(user::me::update_email_notifications),
        )
        .route("/api/v1/summary", get(summary::summary))
        .route(
            "/api/v1/confirm/:email_token",
            put(user::me::confirm_user_email),
        )
        .route(
            "/api/v1/users/:user_id/resend",
            put(user::regenerate_token_and_send),
        )
        .route(
            "/api/v1/site_metadata",
            get(site_metadata::show_deployed_sha),
        )
        // Session management
        .route("/api/private/session/begin", get(user::session::begin))
        .route(
            "/api/private/session/authorize",
            get(user::session::authorize),
        )
        .route("/api/private/session", delete(user::session::logout))
        // Metrics
        .route("/api/private/metrics/:kind", get(metrics::prometheus))
        // Crate ownership invitations management in the frontend
        .route(
            "/api/private/crate_owner_invitations",
            get(crate_owner_invitation::private_list),
        )
        // Alerts from GitHub scanning for exposed API tokens
        .route(
            "/api/github/secret-scanning/verify",
            post(github::secret_scanning::verify),
        );

    // Only serve the local checkout of the git index in development mode.
    // In production, for crates.io, cargo gets the index from
    // https://github.com/rust-lang/crates.io-index directly
    // or from the sparse index CDN https://index.crates.io.
    if state.config.env() == Env::Development {
        router = router.route(
            "/git/index/*path",
            get(git::http_backend).post(git::http_backend),
        );
    }

    router
        .route("/api/openapi.json", get(|| async { Json(openapi) }))
        .fallback(|method: Method| async move {
            match method {
                Method::HEAD => StatusCode::NOT_FOUND.into_response(),
                _ => not_found().into_response(),
            }
        })
        .with_state(state)
}
