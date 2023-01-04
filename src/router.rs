use axum::handler::Handler as AxumHandler;
use axum::middleware::from_fn_with_state;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::Router;
use conduit::{ConduitRequest, Handler, HandlerResult};
use conduit_axum::{CauseField, ConduitAxumHandler};

use crate::app::AppState;
use crate::controllers::*;
use crate::middleware::app::add_app_state_extension;
use crate::util::errors::{not_found, AppError};
use crate::util::EndpointResult;
use crate::Env;

pub fn build_axum_router(state: AppState) -> Router {
    let conduit = |handler| {
        ConduitAxumHandler::wrap(C(handler))
            .layer(from_fn_with_state(state.clone(), add_app_state_extension))
    };

    let mut router = Router::new()
        // Route used by both `cargo search` and the frontend
        .route("/api/v1/crates", get(conduit(krate::search::search)))
        // Routes used by `cargo`
        .route("/api/v1/crates/new", put(conduit(krate::publish::publish)))
        .route(
            "/api/v1/crates/:crate_id/owners",
            get(conduit(krate::owners::owners))
                .put(conduit(krate::owners::add_owners))
                .delete(conduit(krate::owners::remove_owners)),
        )
        .route(
            "/api/v1/crates/:crate_id/:version/yank",
            delete(conduit(version::yank::yank)),
        )
        .route(
            "/api/v1/crates/:crate_id/:version/unyank",
            put(conduit(version::yank::unyank)),
        )
        .route(
            "/api/v1/crates/:crate_id/:version/download",
            get(conduit(version::downloads::download)),
        )
        // Routes that appear to be unused
        .route("/api/v1/versions", get(conduit(version::deprecated::index)))
        .route(
            "/api/v1/versions/:version_id",
            get(conduit(version::deprecated::show_by_id)),
        )
        // Routes used by the frontend
        .route(
            "/api/v1/crates/:crate_id",
            get(conduit(krate::metadata::show)),
        )
        .route(
            "/api/v1/crates/:crate_id/:version",
            get(conduit(version::metadata::show)),
        )
        .route(
            "/api/v1/crates/:crate_id/:version/readme",
            get(conduit(krate::metadata::readme)),
        )
        .route(
            "/api/v1/crates/:crate_id/:version/dependencies",
            get(conduit(version::metadata::dependencies)),
        )
        .route(
            "/api/v1/crates/:crate_id/:version/downloads",
            get(conduit(version::downloads::downloads)),
        )
        .route(
            "/api/v1/crates/:crate_id/:version/authors",
            get(conduit(version::metadata::authors)),
        )
        .route(
            "/api/v1/crates/:crate_id/downloads",
            get(conduit(krate::downloads::downloads)),
        )
        .route(
            "/api/v1/crates/:crate_id/versions",
            get(conduit(krate::metadata::versions)),
        )
        .route(
            "/api/v1/crates/:crate_id/follow",
            put(conduit(krate::follow::follow)).delete(conduit(krate::follow::unfollow)),
        )
        .route(
            "/api/v1/crates/:crate_id/following",
            get(conduit(krate::follow::following)),
        )
        .route(
            "/api/v1/crates/:crate_id/owner_team",
            get(conduit(krate::owners::owner_team)),
        )
        .route(
            "/api/v1/crates/:crate_id/owner_user",
            get(conduit(krate::owners::owner_user)),
        )
        .route(
            "/api/v1/crates/:crate_id/reverse_dependencies",
            get(conduit(krate::metadata::reverse_dependencies)),
        )
        .route("/api/v1/keywords", get(conduit(keyword::index)))
        .route("/api/v1/keywords/:keyword_id", get(keyword::show))
        .route("/api/v1/categories", get(conduit(category::index)))
        .route(
            "/api/v1/categories/:category_id",
            get(conduit(category::show)),
        )
        .route("/api/v1/category_slugs", get(conduit(category::slugs)))
        .route(
            "/api/v1/users/:user_id",
            get(conduit(user::other::show)).put(conduit(user::me::update_user)),
        )
        .route(
            "/api/v1/users/:user_id/stats",
            get(conduit(user::other::stats)),
        )
        .route("/api/v1/teams/:team_id", get(conduit(team::show_team)))
        .route("/api/v1/me", get(conduit(user::me::me)))
        .route("/api/v1/me/updates", get(conduit(user::me::updates)))
        .route(
            "/api/v1/me/tokens",
            get(conduit(token::list)).put(conduit(token::new)),
        )
        .route("/api/v1/me/tokens/:id", delete(conduit(token::revoke)))
        .route(
            "/api/v1/tokens/current",
            delete(conduit(token::revoke_current)),
        )
        .route(
            "/api/v1/me/crate_owner_invitations",
            get(conduit(crate_owner_invitation::list)),
        )
        .route(
            "/api/v1/me/crate_owner_invitations/:crate_id",
            put(conduit(crate_owner_invitation::handle_invite)),
        )
        .route(
            "/api/v1/me/crate_owner_invitations/accept/:token",
            put(conduit(crate_owner_invitation::handle_invite_with_token)),
        )
        .route(
            "/api/v1/me/email_notifications",
            put(conduit(user::me::update_email_notifications)),
        )
        .route("/api/v1/summary", get(conduit(krate::metadata::summary)))
        .route(
            "/api/v1/confirm/:email_token",
            put(conduit(user::me::confirm_user_email)),
        )
        .route(
            "/api/v1/users/:user_id/resend",
            put(conduit(user::me::regenerate_token_and_send)),
        )
        .route(
            "/api/v1/site_metadata",
            get(site_metadata::show_deployed_sha),
        )
        // Session management
        .route(
            "/api/private/session/begin",
            get(conduit(user::session::begin)),
        )
        .route(
            "/api/private/session/authorize",
            get(conduit(user::session::authorize)),
        )
        .route(
            "/api/private/session",
            delete(conduit(user::session::logout)),
        )
        // Metrics
        .route(
            "/api/private/metrics/:kind",
            get(conduit(metrics::prometheus)),
        )
        // Crate ownership invitations management in the frontend
        .route(
            "/api/private/crate_owner_invitations",
            get(conduit(crate_owner_invitation::private_list)),
        )
        // Alerts from GitHub scanning for exposed API tokens
        .route(
            "/api/github/secret-scanning/verify",
            post(conduit(github::secret_scanning::verify)),
        );

    // Only serve the local checkout of the git index in development mode.
    // In production, for crates.io, cargo gets the index from
    // https://github.com/rust-lang/crates.io-index directly.
    if state.config.env() == Env::Development {
        router = router.route(
            "/git/index/*path",
            get(git::http_backend).post(git::http_backend),
        );
    }

    router
        .fallback(|| async { not_found().into_response() })
        .with_state(state)
}

struct C(pub fn(&mut ConduitRequest) -> EndpointResult);

impl Handler for C {
    fn call(&self, req: &mut ConduitRequest) -> HandlerResult {
        let C(f) = *self;
        match f(req) {
            Ok(resp) => Ok(resp),
            Err(e) => {
                let mut response = e.response();

                if let Some(cause) = e.cause() {
                    response
                        .extensions_mut()
                        .insert(CauseField(cause.to_string()));
                }

                Ok(response)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::log_request::CustomMetadata;
    use crate::util::errors::{bad_request, cargo_err, forbidden, internal, not_found, AppError};
    use crate::util::EndpointResult;

    use conduit_test::MockRequest;
    use diesel::result::Error as DieselError;
    use http::StatusCode;

    fn err<E: AppError>(err: E) -> EndpointResult {
        Err(Box::new(err))
    }

    #[test]
    fn http_error_responses() {
        let mut req = MockRequest::new(::conduit::Method::GET, "/").into_inner();
        req.extensions_mut().insert(CustomMetadata::default());

        // Types for handling common error status codes
        assert_eq!(
            C(|_| Err(bad_request(""))).call(&mut req).unwrap().status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            C(|_| Err(forbidden())).call(&mut req).unwrap().status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            C(|_| Err(DieselError::NotFound.into()))
                .call(&mut req)
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            C(|_| Err(not_found())).call(&mut req).unwrap().status(),
            StatusCode::NOT_FOUND
        );

        // cargo_err errors are returned as 200 so that cargo displays this nicely on the command line
        assert_eq!(
            C(|_| Err(cargo_err(""))).call(&mut req).unwrap().status(),
            StatusCode::OK
        );

        // Inner errors are captured for logging when wrapped by a user facing error
        let response = C(|_| {
            Err("-1"
                .parse::<u8>()
                .map_err(|err| err.chain(internal("middle error")))
                .map_err(|err| err.chain(bad_request("outer user facing error")))
                .unwrap_err())
        })
        .call(&mut req)
        .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.extensions().get::<CauseField>().unwrap().0,
            "middle error caused by invalid digit found in string"
        );

        // All other error types are converted to internal server errors
        assert_eq!(
            C(|_| Err(internal(""))).call(&mut req).unwrap().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            C(|_| err::<::serde_json::Error>(::serde::de::Error::custom("ExpectedColon")))
                .call(&mut req)
                .unwrap()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            C(|_| err(::std::io::Error::new(::std::io::ErrorKind::Other, "")))
                .call(&mut req)
                .unwrap()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
