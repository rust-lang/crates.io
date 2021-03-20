mod prelude {
    pub use conduit::{box_error, header, Body, Handler, RequestExt, Response, StatusCode};
    pub use conduit_middleware::{AfterResult, AroundMiddleware, BeforeResult, Middleware};
}

use self::app::AppMiddleware;
use self::debug::*;
use self::ember_html::EmberHtml;
use self::head::Head;
use self::known_error_to_json::KnownErrorToJson;
use self::log_connection_pool_status::LogConnectionPoolStatus;
use self::static_or_continue::StaticOrContinue;

pub mod app;
mod balance_capacity;
mod block_traffic;
mod debug;
mod ember_html;
mod ensure_well_formed_500;
mod head;
mod known_error_to_json;
mod log_connection_pool_status;
pub mod log_request;
mod normalize_path;
mod require_user_agent;
mod static_or_continue;

use conduit_conditional_get::ConditionalGet;
use conduit_cookie::{Middleware as Cookie, SessionMiddleware};
use conduit_middleware::MiddlewareBuilder;
use conduit_router::RouteBuilder;

use std::env;
use std::sync::Arc;

use crate::{App, Env};

pub fn build_middleware(app: Arc<App>, endpoints: RouteBuilder) -> MiddlewareBuilder {
    let mut m = MiddlewareBuilder::new(endpoints);
    let config = app.config.clone();
    let env = config.env;

    if env != Env::Test {
        m.add(ensure_well_formed_500::EnsureWellFormed500);
        m.add(log_request::LogRequests::default());
    }

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
        cookie::Key::derive_from(app.session_key.as_bytes()),
        env == Env::Production,
    ));

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
    if let Ok(capacity) = env::var("DB_POOL_SIZE") {
        if let Ok(capacity) = capacity.parse() {
            if capacity >= 10 {
                println!(
                    "Enabling BalanceCapacity middleware with {} pool capacity",
                    capacity
                );
                m.around(balance_capacity::BalanceCapacity::new(capacity))
            } else {
                println!("BalanceCapacity middleware not enabled. DB_POOL_SIZE is too low.");
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

    for (header, blocked_values) in config.blocked_traffic {
        m.around(block_traffic::BlockTraffic::new(header, blocked_values));
    }

    m.around(require_user_agent::RequireUserAgent::default());

    m
}
