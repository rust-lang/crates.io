mod prelude {
    pub use super::log_request::add_custom_metadata;
    pub use conduit::{box_error, header, Body, Handler, RequestExt, Response, StatusCode};
    pub use conduit_middleware::{AfterResult, AroundMiddleware, BeforeResult, Middleware};
}

use self::app::AppMiddleware;
use self::debug::*;
use self::ember_html::EmberHtml;
use self::head::Head;
use self::known_error_to_json::KnownErrorToJson;
use self::log_connection_pool_status::LogConnectionPoolStatus;
use self::response_timing::ResponseTiming;
use self::static_or_continue::StaticOrContinue;
use self::update_metrics::UpdateMetrics;

pub mod app;
mod balance_capacity;
mod block_traffic;
mod debug;
mod ember_html;
mod head;
mod known_error_to_json;
mod log_connection_pool_status;
pub mod log_request;
mod normalize_path;
mod require_user_agent;
pub mod response_timing;
mod static_or_continue;
mod update_metrics;

use conduit_conditional_get::ConditionalGet;
use conduit_cookie::{Middleware as Cookie, SessionMiddleware};
use conduit_middleware::MiddlewareBuilder;
use conduit_router::RouteBuilder;

use std::env;
use std::sync::Arc;

use crate::sentry::SentryMiddleware;
use crate::{App, Env};

pub fn build_middleware(app: Arc<App>, endpoints: RouteBuilder) -> MiddlewareBuilder {
    let mut m = MiddlewareBuilder::new(endpoints);
    let env = app.config.env();
    let blocked_traffic = app.config.blocked_traffic.clone();

    if env != Env::Test {
        m.add(SentryMiddleware::default());
        m.add(log_request::LogRequests::default());
    }

    m.add(ResponseTiming::default());

    if env == Env::Development {
        // Optionally print debug information for each request
        // To enable, set the environment variable: `RUST_LOG=cargo_registry::middleware=debug`
        m.add(Debug);
    }

    if env::var_os("LOG_CONNECTION_POOL_STATUS").is_some() {
        m.add(LogConnectionPoolStatus::new(&app));
    }

    m.add(normalize_path::NormalizePath);
    m.add(ConditionalGet);

    m.add(Cookie::new());
    m.add(SessionMiddleware::new(
        "cargo_session",
        cookie::Key::derive_from(app.session_key().as_bytes()),
        env == Env::Production,
    ));

    m.add(AppMiddleware::new(app));
    m.add(KnownErrorToJson);

    // This is added *after* AppMiddleware to make sure the app is available.
    m.add(UpdateMetrics);

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
                println!("Enabling BalanceCapacity middleware with {capacity} pool capacity");
                m.around(balance_capacity::BalanceCapacity::new(capacity))
            } else {
                println!(
                    "BalanceCapacity middleware not enabled. DB_PRIMARY_POOL_SIZE is too low."
                );
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
