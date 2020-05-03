mod prelude {
    pub use conduit::{box_error, header, Body, Handler, RequestExt, Response, StatusCode};
    pub use conduit_middleware::{AfterResult, AroundMiddleware, BeforeResult, Middleware};
}

use self::app::AppMiddleware;
use self::current_user::CaptureUserIdFromCookie;
use self::debug::*;
use self::ember_html::EmberHtml;
use self::head::Head;
use self::log_connection_pool_status::LogConnectionPoolStatus;
use self::static_or_continue::StaticOrContinue;

pub mod app;
mod block_traffic;
pub mod current_user;
mod debug;
mod ember_html;
mod ensure_well_formed_500;
mod head;
mod log_connection_pool_status;
pub mod log_request;
mod require_user_agent;
mod static_or_continue;

use conduit_conditional_get::ConditionalGet;
use conduit_cookie::{Middleware as Cookie, SessionMiddleware};
use conduit_middleware::MiddlewareBuilder;

use std::env;
use std::sync::Arc;

use crate::router::R404;
use crate::{App, Env};

pub fn build_middleware(app: Arc<App>, endpoints: R404) -> MiddlewareBuilder {
    let mut m = MiddlewareBuilder::new(endpoints);
    let config = app.config.clone();
    let env = config.env;

    if env != Env::Test {
        m.add(ensure_well_formed_500::EnsureWellFormed500);
        m.add(log_request::LogRequests::default());
    }

    if env == Env::Development {
        // Print a log for each request.
        m.add(Debug);
        // Locally serve crates and readmes
        m.around(StaticOrContinue::new("local_uploads"));
    }

    if env::var_os("DEBUG_REQUESTS").is_some() {
        m.add(DebugRequest);
    }

    if env::var_os("LOG_CONNECTION_POOL_STATUS").is_some() {
        m.add(LogConnectionPoolStatus::new(&app));
    }

    m.add(ConditionalGet);

    m.add(Cookie::new());
    m.add(SessionMiddleware::new(
        "cargo_session",
        cookie::Key::from_master(app.session_key.as_bytes()),
        env == Env::Production,
    ));

    m.add(AppMiddleware::new(app));

    // Parse and save the user_id from the session cookie as part of the authentication logic
    m.add(CaptureUserIdFromCookie);

    // Note: The following `m.around()` middleware is run from bottom to top

    // Serve the static files in the *dist* directory, which are the frontend assets.
    // Not needed for the backend tests.
    if env != Env::Test {
        m.around(EmberHtml::new("dist"));
        m.around(StaticOrContinue::new("dist"));
    }

    m.around(Head::default());

    for (header, blocked_values) in config.blocked_traffic {
        m.around(block_traffic::BlockTraffic::new(header, blocked_values));
    }

    m.around(require_user_agent::RequireUserAgent::default());

    m
}
