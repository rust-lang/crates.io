pub mod app;
mod block_traffic;
pub mod cargo_compat;
mod common_headers;
mod debug;
mod ember_html;
pub mod log_request;
pub mod normalize_path;
pub mod real_ip;
mod require_user_agent;
pub mod session;
mod static_or_continue;
mod update_metrics;

use ::sentry::integrations::tower as sentry_tower;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::Router;
use axum_extra::either::Either;
use axum_extra::middleware::option_layer;
use std::time::Duration;
use tower::layer::util::Identity;
use tower_http::add_extension::AddExtensionLayer;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::{CompressionLayer, CompressionLevel};
use tower_http::timeout::{RequestBodyTimeoutLayer, TimeoutLayer};

use crate::app::AppState;
use crate::Env;

pub fn apply_axum_middleware(state: AppState, router: Router<()>) -> Router {
    let config = &state.config;
    let env = config.env();

    // The middleware stacks here have been split for compile performance
    // reasons. The type signatures of the `ServiceBuilder` were approaching
    // dozens of kilobytes, triggering exponential behaviors in the compiler.
    // Splitting the stacks into two smaller stacks seem to avoid this problem.
    //
    // See also https://github.com/rust-lang/crates.io/pull/7443.

    let middlewares_1 = tower::ServiceBuilder::new()
        .layer(sentry_tower::NewSentryLayer::new_from_top())
        .layer(sentry_tower::SentryHttpLayer::with_transaction())
        .layer(from_fn(self::real_ip::middleware))
        .layer(from_fn(log_request::log_requests))
        .layer(CatchPanicLayer::new())
        .layer(from_fn_with_state(
            state.clone(),
            update_metrics::update_metrics,
        ))
        // Optionally print debug information for each request
        // To enable, set the environment variable: `RUST_LOG=crates_io::middleware=debug`
        .layer(conditional_layer(env == Env::Development, || {
            from_fn(debug::debug_requests)
        }));

    let middlewares_2 = tower::ServiceBuilder::new()
        .layer(from_fn_with_state(
            state.config.cargo_compat_status_code_config,
            cargo_compat::middleware,
        ))
        .layer(from_fn_with_state(state.clone(), session::attach_session))
        .layer(from_fn_with_state(
            state.clone(),
            require_user_agent::require_user_agent,
        ))
        .layer(from_fn_with_state(state.clone(), block_traffic::middleware))
        .layer(from_fn_with_state(
            state.clone(),
            common_headers::add_common_headers,
        ))
        .layer(conditional_layer(env == Env::Development, || {
            from_fn(static_or_continue::serve_local_uploads)
        }))
        .layer(conditional_layer(config.serve_dist, || {
            from_fn(static_or_continue::serve_dist)
        }))
        .layer(conditional_layer(config.serve_html, || {
            from_fn_with_state(state.clone(), ember_html::serve_html)
        }))
        .layer(AddExtensionLayer::new(state.clone()));

    router
        .layer(middlewares_2)
        .layer(middlewares_1)
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(RequestBodyTimeoutLayer::new(Duration::from_secs(30)))
        .layer(CompressionLayer::new().quality(CompressionLevel::Fastest))
}

pub fn conditional_layer<L, F: FnOnce() -> L>(condition: bool, layer: F) -> Either<L, Identity> {
    option_layer(condition.then(layer))
}
