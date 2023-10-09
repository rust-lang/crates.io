use axum::extract::DefaultBodyLimit;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::Router;
use http::{Method, StatusCode};

use crate::app::AppState;
use crate::controllers::*;
use crate::util::errors::not_found;
use crate::Env;

const MAX_PUBLISH_CONTENT_LENGTH: usize = 128 * 1024 * 1024; // 128 MB

pub fn build_axum_router(state: AppState) -> Router {
    let mut router = Router::new()
        // Route used by both `cargo search` and the frontend
        .route("/api/v1/crates", get(krate::search::search))
        // Routes used by `cargo`
        .route(
            "/api/v1/crates/new",
            put(krate::publish::publish).layer(DefaultBodyLimit::max(MAX_PUBLISH_CONTENT_LENGTH)),
        )
        .route(
            "/api/v1/crates/:crate_id/owners",
            get(krate::owners::owners)
                .put(krate::owners::add_owners)
                .delete(krate::owners::remove_owners),
        )
        .route(
            "/api/v1/crates/:crate_id/:version/yank",
            delete(version::yank::yank),
        )
        .route(
            "/api/v1/crates/:crate_id/:version/unyank",
            put(version::yank::unyank),
        )
        .route(
            "/api/v1/crates/:crate_id/:version/download",
            get(version::downloads::download),
        )
        // Routes that appear to be unused
        .route("/api/v1/versions", get(version::deprecated::index))
        .route(
            "/api/v1/versions/:version_id",
            get(version::deprecated::show_by_id),
        )
        // Routes used by the frontend
        .route("/api/v1/crates/:crate_id", get(krate::metadata::show))
        .route(
            "/api/v1/crates/:crate_id/:version",
            get(version::metadata::show),
        )
        .route(
            "/api/v1/crates/:crate_id/:version/readme",
            get(krate::metadata::readme),
        )
        .route(
            "/api/v1/crates/:crate_id/:version/dependencies",
            get(version::metadata::dependencies),
        )
        .route(
            "/api/v1/crates/:crate_id/:version/downloads",
            get(version::downloads::downloads),
        )
        .route(
            "/api/v1/crates/:crate_id/:version/authors",
            get(version::metadata::authors),
        )
        .route(
            "/api/v1/crates/:crate_id/downloads",
            get(krate::downloads::downloads),
        )
        .route(
            "/api/v1/crates/:crate_id/versions",
            get(krate::metadata::versions),
        )
        .route(
            "/api/v1/crates/:crate_id/follow",
            put(krate::follow::follow).delete(krate::follow::unfollow),
        )
        .route(
            "/api/v1/crates/:crate_id/following",
            get(krate::follow::following),
        )
        .route(
            "/api/v1/crates/:crate_id/owner_team",
            get(krate::owners::owner_team),
        )
        .route(
            "/api/v1/crates/:crate_id/owner_user",
            get(krate::owners::owner_user),
        )
        .route(
            "/api/v1/crates/:crate_id/reverse_dependencies",
            get(krate::metadata::reverse_dependencies),
        )
        .route("/api/v1/keywords", get(keyword::index))
        .route("/api/v1/keywords/:keyword_id", get(keyword::show))
        .route("/api/v1/categories", get(category::index))
        .route("/api/v1/categories/:category_id", get(category::show))
        .route("/api/v1/category_slugs", get(category::slugs))
        .route(
            "/api/v1/users/:user_id",
            get(user::other::show).put(user::me::update_user),
        )
        .route("/api/v1/users/:user_id/stats", get(user::other::stats))
        .route("/api/v1/teams/:team_id", get(team::show_team))
        .route("/api/v1/me", get(user::me::me))
        .route("/api/v1/me/updates", get(user::me::updates))
        .route("/api/v1/me/tokens", get(token::list).put(token::new))
        .route("/api/v1/me/tokens/:id", delete(token::revoke))
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
        .route("/api/v1/summary", get(krate::metadata::summary))
        .route(
            "/api/v1/confirm/:email_token",
            put(user::me::confirm_user_email),
        )
        .route(
            "/api/v1/users/:user_id/resend",
            put(user::me::regenerate_token_and_send),
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
        .fallback(|method: Method| async move {
            match method {
                Method::HEAD => StatusCode::NOT_FOUND.into_response(),
                _ => not_found().into_response(),
            }
        })
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use crate::middleware::log_request::CauseField;
    use crate::util::errors::{
        bad_request, cargo_err, forbidden, internal, not_found, AppError, BoxedAppError,
    };
    use axum::response::IntoResponse;
    use diesel::result::Error as DieselError;
    use http::StatusCode;
    use serde::de::Error;

    #[test]
    fn http_error_responses() {
        // Types for handling common error status codes
        assert_eq!(bad_request("").response().status(), StatusCode::BAD_REQUEST);
        assert_eq!(forbidden().response().status(), StatusCode::FORBIDDEN);
        assert_eq!(
            BoxedAppError::from(DieselError::NotFound)
                .response()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(not_found().response().status(), StatusCode::NOT_FOUND);

        // cargo_err errors are returned as 200 so that cargo displays this nicely on the command line
        assert_eq!(cargo_err("").response().status(), StatusCode::OK);

        // Inner errors are captured for logging when wrapped by a user facing error
        let response = "-1"
            .parse::<u8>()
            .map_err(|err| err.chain(internal("middle error")))
            .map_err(|err| err.chain(bad_request("outer user facing error")))
            .unwrap_err()
            .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.extensions().get::<CauseField>().unwrap().0,
            "middle error caused by invalid digit found in string"
        );

        // All other error types are converted to internal server errors
        assert_eq!(
            internal("").response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            BoxedAppError::from(serde_json::Error::custom("ExpectedColon"))
                .response()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            BoxedAppError::from(::std::io::Error::new(::std::io::ErrorKind::Other, ""))
                .response()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
