mod prelude {
    pub use super::log_request::add_custom_metadata;
    pub use conduit::{box_error, Body, Handler, RequestExt};
    pub use conduit_middleware::{AfterResult, AroundMiddleware, BeforeResult, Middleware};
    pub use http::{header, Response, StatusCode};
}

use self::app::AppMiddleware;
use self::ember_html::EmberHtml;
use self::head::Head;
use self::known_error_to_json::KnownErrorToJson;
use self::static_or_continue::StaticOrContinue;

pub mod app;
mod balance_capacity;
mod block_traffic;
mod debug;
mod ember_html;
mod head;
mod known_error_to_json;
pub mod log_request;
pub mod normalize_path;
mod require_user_agent;
pub mod session;
mod static_or_continue;
mod update_metrics;

use conduit_conditional_get::ConditionalGet;
use conduit_middleware::MiddlewareBuilder;
use conduit_router::RouteBuilder;

use ::sentry::integrations::tower as sentry_tower;
use axum::error_handling::HandleErrorLayer;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::Router;
use std::env;
use std::sync::Arc;

use crate::app::AppState;
use crate::{App, Env};

pub fn apply_axum_middleware(state: AppState, router: Router) -> Router {
    type Request = http::Request<axum::body::Body>;

    let env = state.config.env();

    let middleware = tower::ServiceBuilder::new()
        .layer(sentry_tower::NewSentryLayer::<Request>::new_from_top())
        .layer(sentry_tower::SentryHttpLayer::with_transaction())
        .layer(from_fn(log_request::log_requests))
        .layer(from_fn_with_state(
            state.clone(),
            update_metrics::update_metrics,
        ))
        // The following layer is unfortunately necessary for `option_layer()` to work
        .layer(HandleErrorLayer::new(dummy_error_handler))
        // Optionally print debug information for each request
        // To enable, set the environment variable: `RUST_LOG=cargo_registry::middleware=debug`
        .option_layer((env == Env::Development).then(|| from_fn(debug::debug_requests)))
        .layer(from_fn_with_state(state, session::attach_session));

    router.layer(middleware)
}

/// This function is only necessary because `.option_layer()` changes the error type
/// and we need to change it back. Since the axum middleware has no way of returning
/// an actual error this function should never actually be called.
async fn dummy_error_handler(_err: axum::BoxError) -> http::StatusCode {
    http::StatusCode::INTERNAL_SERVER_ERROR
}

pub fn build_middleware(app: Arc<App>, endpoints: RouteBuilder) -> MiddlewareBuilder {
    let mut m = MiddlewareBuilder::new(endpoints);
    let env = app.config.env();
    let blocked_traffic = app.config.blocked_traffic.clone();

    m.add(log_request::LogRequests::default());

    m.add(ConditionalGet);

    m.add(AppMiddleware::new(app));
    m.add(KnownErrorToJson);

    // Note: The following `m.around()` middleware is run from bottom to top

    // This is currently the final middleware to run. If a middleware layer requires a database
    // connection, it should be run after this middleware so that the potential pool usage can be
    // tracked here.
    //
    // In production we currently have 2 equally sized pools (primary and a read-only replica).
    // Because such a large portion of production traffic is for download requests (which update
    // download counts), we consider only the primary pool here.
    if let Ok(capacity) = env::var("DB_PRIMARY_POOL_SIZE") {
        if let Ok(capacity) = capacity.parse() {
            if capacity >= 10 {
                info!(?capacity, "Enabling BalanceCapacity middleware");
                m.around(balance_capacity::BalanceCapacity::new(capacity))
            } else {
                info!("BalanceCapacity middleware not enabled. DB_PRIMARY_POOL_SIZE is too low.");
            }
        }
    }

    // Serve the static files in the *dist* directory, which are the frontend assets.
    // Not needed for the backend tests.
    if env != Env::Test {
        m.around(EmberHtml::new("dist"));
        m.around(StaticOrContinue::new("dist"));
    }

    if env == Env::Development {
        // Locally serve crates and readmes
        m.around(StaticOrContinue::new("local_uploads"));
    }

    m.around(Head::default());

    for (header, blocked_values) in blocked_traffic {
        m.around(block_traffic::BlockTraffic::new(header, blocked_values));
    }

    m.around(require_user_agent::RequireUserAgent::default());

    m
}
