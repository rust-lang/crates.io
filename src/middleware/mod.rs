mod prelude {
    pub use std::error::Error;
    pub use conduit::{Handler, Request, Response};
    pub use conduit_middleware::{AroundMiddleware, Middleware};
}

pub use self::app::AppMiddleware;
pub use self::current_user::CurrentUser;
pub use self::debug::*;
pub use self::ember_index_rewrite::EmberIndexRewrite;
pub use self::head::Head;
pub use self::security_headers::SecurityHeaders;
pub use self::static_or_continue::StaticOrContinue;

pub mod app;
pub mod current_user;
mod debug;
mod ember_index_rewrite;
mod head;
mod security_headers;
mod static_or_continue;

use conduit_middleware::MiddlewareBuilder;
use conduit_conditional_get::ConditionalGet;
use conduit_cookie::{Middleware as Cookie, SessionMiddleware};
use conduit_log_requests::LogRequests;

use std::env;
use std::sync::Arc;
use cookie;
use log;

use {App, Env};
use router::R404;

pub fn build_middleware(app: Arc<App>, endpoints: R404) -> MiddlewareBuilder {
    let mut m = MiddlewareBuilder::new(endpoints);

    let env = app.config.env;
    if env == Env::Development {
        // Print a log for each request.
        m.add(Debug);
        // Locally serve crates and readmes
        m.around(StaticOrContinue::new("local_uploads"));
    }

    if env::var_os("DEBUG_REQUESTS").is_some() {
        m.add(DebugRequest);
    }

    if env != Env::Test {
        m.add(LogRequests(log::LogLevel::Info));
    }

    m.add(ConditionalGet);

    m.add(Cookie::new());
    m.add(SessionMiddleware::new(
        "cargo_session",
        cookie::Key::from_master(app.session_key.as_bytes()),
        env == Env::Production,
    ));

    if env == Env::Production {
        m.add(SecurityHeaders::new(&app.config.uploader));
    }
    m.add(AppMiddleware::new(app));

    // Sets the current user on each request.
    m.add(CurrentUser);

    // Serve the static files in the *dist* directory, which are the frontend assets.
    // Not needed for the backend tests.
    if env != Env::Test {
        m.around(StaticOrContinue::new("dist"));
        m.around(EmberIndexRewrite::default());
        // Note: around middleware is run from bottom to top, so the rewrite occurs first
    }

    m.around(Head::default());

    m
}
