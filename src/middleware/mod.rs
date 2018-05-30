mod prelude {
    pub use conduit::{Handler, Request, Response};
    pub use conduit_middleware::{AroundMiddleware, Middleware};
    pub use std::error::Error;
}

pub use self::app::AppMiddleware;
pub use self::current_user::CurrentUser;
pub use self::debug::*;
pub use self::ember_index_rewrite::EmberIndexRewrite;
pub use self::head::Head;
pub use self::security_headers::SecurityHeaders;
pub use self::static_or_continue::StaticOrContinue;

pub mod app;
mod blacklist_ips;
pub mod current_user;
mod debug;
mod ember_index_rewrite;
mod ensure_well_formed_500;
mod head;
mod log_request;
mod security_headers;
mod static_or_continue;

use conduit_conditional_get::ConditionalGet;
use conduit_cookie::{Middleware as Cookie, SessionMiddleware};
use conduit_middleware::MiddlewareBuilder;

use cookie;
use std::env;
use std::sync::Arc;

use router::R404;
use {App, Env};

pub fn build_middleware(app: Arc<App>, endpoints: R404) -> MiddlewareBuilder {
    let mut m = MiddlewareBuilder::new(endpoints);
    let env = app.config.env;

    if env != Env::Test {
        m.add(ensure_well_formed_500::EnsureWellFormed500);
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
        m.around(StaticOrContinue::new("dist"));
        // Note: around middleware is run from bottom to top, so the rewrite occurs first
    }

    m.around(Head::default());

    if let Ok(ip_list) = env::var("BLACKLISTED_IPS") {
        let ips = ip_list.split(',').map(String::from).collect();
        m.around(blacklist_ips::BlockIps::new(ips));
    }

    if env != Env::Test {
        m.around(log_request::LogRequests::default());
    }

    m
}
