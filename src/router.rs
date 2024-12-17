use axum::response::IntoResponse;
use axum::routing::{get, post};
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
        .routes(routes!(krate::search::list_crates))
        // Routes used by `cargo`
        .routes(routes!(
            krate::publish::publish,
            krate::metadata::find_new_crate
        ))
        .routes(routes!(
            krate::owners::list_owners,
            krate::owners::add_owners,
            krate::owners::remove_owners
        ))
        .routes(routes!(version::yank::yank_version))
        .routes(routes!(version::yank::unyank_version))
        .routes(routes!(version::downloads::download_version))
        // Routes used by the frontend
        .routes(routes!(
            krate::metadata::find_crate,
            krate::delete::delete_crate
        ))
        .routes(routes!(
            version::metadata::find_version,
            version::metadata::update_version
        ))
        .routes(routes!(version::readme::get_version_readme))
        .routes(routes!(version::dependencies::get_version_dependencies))
        .routes(routes!(version::downloads::get_version_downloads))
        .routes(routes!(version::authors::get_version_authors))
        .routes(routes!(krate::downloads::get_crate_downloads))
        .routes(routes!(krate::versions::list_versions))
        .routes(routes!(
            krate::follow::follow_crate,
            krate::follow::unfollow_crate
        ))
        .routes(routes!(krate::follow::get_following_crate))
        .routes(routes!(krate::owners::get_team_owners))
        .routes(routes!(krate::owners::get_user_owners))
        .routes(routes!(krate::rev_deps::list_reverse_dependencies))
        .routes(routes!(keyword::list_keywords))
        .routes(routes!(keyword::find_keyword))
        .routes(routes!(category::list_categories))
        .routes(routes!(category::find_category))
        .routes(routes!(category::list_category_slugs))
        .routes(routes!(user::other::find_user, user::update::update_user))
        .routes(routes!(user::other::get_user_stats))
        .routes(routes!(team::find_team))
        .routes(routes!(user::me::get_authenticated_user))
        .routes(routes!(user::me::get_authenticated_user_updates))
        .routes(routes!(token::list_api_tokens, token::create_api_token))
        .routes(routes!(token::find_api_token, token::revoke_api_token))
        .routes(routes!(token::revoke_current_api_token))
        .routes(routes!(
            crate_owner_invitation::list_crate_owner_invitations_for_user
        ))
        .routes(routes!(
            crate_owner_invitation::list_crate_owner_invitations
        ))
        .routes(routes!(
            crate_owner_invitation::handle_crate_owner_invitation
        ))
        .routes(routes!(
            crate_owner_invitation::accept_crate_owner_invitation_with_token
        ))
        .routes(routes!(
            user::email_notifications::update_email_notifications
        ))
        .routes(routes!(summary::get_summary))
        .routes(routes!(user::email_verification::confirm_user_email))
        .routes(routes!(user::email_verification::resend_email_verification))
        .routes(routes!(site_metadata::get_site_metadata))
        // Session management
        .routes(routes!(session::begin_session))
        .routes(routes!(session::authorize_session))
        .routes(routes!(session::end_session))
        .split_for_parts();

    let mut router = router
        // Metrics
        .route("/api/private/metrics/:kind", get(metrics::prometheus))
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
