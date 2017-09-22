//! This crate implements the backend server for https://crates.io/
//!
//! All implemented routes are defined in the [middleware](fn.middleware.html) function and
//! implemented in the [category](category/index.html), [keyword](keyword/index.html),
//! [krate](krate/index.html), [user](user/index.html) and [version](version/index.html) modules.
#![deny(warnings)]
#![deny(missing_debug_implementations, missing_copy_implementations)]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![recursion_limit="128"]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate ammonia;
extern crate chrono;
extern crate comrak;
extern crate curl;
extern crate diesel_full_text_search;
extern crate dotenv;
extern crate flate2;
extern crate git2;
extern crate hex;
extern crate license_exprs;
extern crate oauth2;
extern crate openssl;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate rand;
extern crate s3;
extern crate semver;
extern crate serde;
extern crate tar;
extern crate time;
extern crate toml;
extern crate url;
extern crate lettre;

extern crate conduit;
extern crate conduit_conditional_get;
extern crate conduit_cookie;
extern crate cookie;
extern crate conduit_git_http_backend;
extern crate conduit_log_requests;
extern crate conduit_middleware;
extern crate conduit_router;
extern crate conduit_static;

pub use app::App;
pub use self::badge::Badge;
pub use self::category::Category;
pub use config::Config;
pub use self::dependency::Dependency;
pub use self::download::VersionDownload;
pub use self::keyword::Keyword;
pub use self::krate::Crate;
pub use self::user::User;
pub use self::version::Version;
pub use self::uploaders::{Uploader, Bomb};

use std::sync::Arc;
use std::error::Error;

use conduit_router::RouteBuilder;
use conduit_middleware::MiddlewareBuilder;

use util::{C, R, R404};

pub mod app;
pub mod badge;
pub mod categories;
pub mod category;
pub mod config;
pub mod crate_owner_invitation;
pub mod db;
pub mod dependency;
pub mod dist;
pub mod download;
pub mod git;
pub mod http;
pub mod keyword;
pub mod krate;
pub mod owner;
pub mod render;
pub mod schema;
pub mod token;
pub mod upload;
pub mod uploaders;
pub mod user;
pub mod util;
pub mod version;
pub mod email;

mod local_upload;
mod pagination;

/// Used for setting different values depending on whether the app is being run in production,
/// in development, or for testing.
///
/// The app's `config.env` value is set in *src/bin/server.rs* to `Production` if the environment
/// variable `HEROKU` is set and `Development` otherwise. `config.env` is set to `Test`
/// unconditionally in *src/test/all.rs*.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Env {
    Development,
    Test,
    Production,
}

/// Used for setting different values depending on the type of registry this instance is.
///
/// `Primary` indicates this instance is a primary registry that is the source of truth for these
/// crates' information. `ReadOnlyMirror` indicates this instanceis a read-only mirror of crate
/// information that exists on another instance.
///
/// The app's `config.mirror` value is set in *src/bin/server.rs* to `ReadOnlyMirror` if the
/// `MIRROR` environment variable is set and to `Primary` otherwise.
///
/// There may be more ways to run crates.io servers in the future, such as a
/// mirror that also has private crates that crates.io does not have.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Replica {
    Primary,
    ReadOnlyMirror,
}

/// Configures routes, sessions, logging, and other middleware.
///
/// Called from *src/bin/server.rs*.
pub fn middleware(app: Arc<App>) -> MiddlewareBuilder {
    let mut api_router = RouteBuilder::new();

    api_router.get("/crates", C(krate::index));
    api_router.get("/crates/:crate_id", C(krate::show));
    api_router.put("/crates/new", C(krate::new));
    api_router.get("/crates/:crate_id/:version", C(version::show));
    api_router.get("/crates/:crate_id/:version/download", C(krate::download));
    api_router.get("/crates/:crate_id/:version/readme", C(krate::readme));
    api_router.get(
        "/crates/:crate_id/:version/dependencies",
        C(version::dependencies),
    );
    api_router.get(
        "/crates/:crate_id/:version/downloads",
        C(version::downloads),
    );
    api_router.get("/crates/:crate_id/:version/authors", C(version::authors));
    // Used to generate download graphs
    api_router.get("/crates/:crate_id/downloads", C(krate::downloads));
    api_router.get("/crates/:crate_id/versions", C(krate::versions));
    api_router.put("/crates/:crate_id/follow", C(krate::follow));
    api_router.delete("/crates/:crate_id/follow", C(krate::unfollow));
    api_router.get("/crates/:crate_id/following", C(krate::following));
    // This endpoint may now be redundant, check frontend to see if it is
    // being used
    api_router.get("/crates/:crate_id/owners", C(krate::owners));
    api_router.get("/crates/:crate_id/owner_team", C(krate::owner_team));
    api_router.get("/crates/:crate_id/owner_user", C(krate::owner_user));
    api_router.put("/crates/:crate_id/owners", C(krate::add_owners));
    api_router.delete("/crates/:crate_id/owners", C(krate::remove_owners));
    api_router.delete("/crates/:crate_id/:version/yank", C(version::yank));
    api_router.put("/crates/:crate_id/:version/unyank", C(version::unyank));
    api_router.get(
        "/crates/:crate_id/reverse_dependencies",
        C(krate::reverse_dependencies),
    );
    api_router.get("/versions", C(version::index));
    api_router.get("/versions/:version_id", C(version::show));
    api_router.get("/keywords", C(keyword::index));
    api_router.get("/keywords/:keyword_id", C(keyword::show));
    api_router.get("/categories", C(category::index));
    api_router.get("/categories/:category_id", C(category::show));
    api_router.get("/category_slugs", C(category::slugs));
    api_router.get("/users/:user_id", C(user::show));
    api_router.put("/users/:user_id", C(user::update_user));
    api_router.get("/users/:user_id/stats", C(user::stats));
    api_router.get("/teams/:team_id", C(user::show_team));
    api_router.get("/me", C(user::me));
    api_router.get("/me/updates", C(user::updates));
    api_router.get("/me/tokens", C(token::list));
    api_router.post("/me/tokens", C(token::new));
    api_router.delete("/me/tokens/:id", C(token::revoke));
    api_router.get(
        "/me/crate_owner_invitations",
        C(crate_owner_invitation::list),
    );
    api_router.put("/me/accept_owner_invite", C(crate_owner_invitation::accept_invite));
    api_router.get("/summary", C(krate::summary));
    api_router.put("/confirm/:email_token", C(user::confirm_user_email));
    api_router.put("/users/:user_id/resend", C(user::regenerate_token_and_send));
    let api_router = Arc::new(R404(api_router));

    let mut router = RouteBuilder::new();

    // Mount the router under the /api/v1 path so we're at least somewhat at the
    // liberty to change things in the future!
    router.get("/api/v1/*path", R(api_router.clone()));
    router.put("/api/v1/*path", R(api_router.clone()));
    router.post("/api/v1/*path", R(api_router.clone()));
    router.head("/api/v1/*path", R(api_router.clone()));
    router.delete("/api/v1/*path", R(api_router));

    router.get("/authorize_url", C(user::github_authorize));
    router.get("/authorize", C(user::github_access_token));
    router.delete("/logout", C(user::logout));

    // Only serve the local checkout of the git index in development mode.
    // In production, for crates.io, cargo gets the index from
    // https://github.com/rust-lang/crates.io-index directly.
    let env = app.config.env;
    if env == Env::Development {
        let s = conduit_git_http_backend::Serve(app.git_repo_checkout.clone());
        let s = Arc::new(s);
        router.get("/git/index/*path", R(s.clone()));
        router.post("/git/index/*path", R(s));
    }

    let mut m = MiddlewareBuilder::new(R404(router));

    if env == Env::Development {
        // DebugMiddleware is defined below to print logs for each request.
        m.add(DebugMiddleware);
        m.around(local_upload::Middleware::default());
    }

    if env != Env::Test {
        m.add(conduit_log_requests::LogRequests(log::LogLevel::Info));
    }

    m.around(util::Head::default());
    m.add(conduit_conditional_get::ConditionalGet);
    m.add(conduit_cookie::Middleware::new());
    m.add(conduit_cookie::SessionMiddleware::new(
        "cargo_session",
        cookie::Key::from_master(app.session_key.as_bytes()),
        env == Env::Production,
    ));
    if env == Env::Production {
        m.add(http::SecurityHeadersMiddleware::new(&app.config.uploader));
    }
    m.add(app::AppMiddleware::new(app));

    // Sets the current user on each request.
    m.add(user::Middleware);

    // Serve the static files in the *dist* directory, which are the frontend assets.
    // Not needed for the backend tests.
    if env != Env::Test {
        m.around(dist::Middleware::default());
    }

    return m;

    struct DebugMiddleware;

    impl conduit_middleware::Middleware for DebugMiddleware {
        fn before(&self, req: &mut conduit::Request) -> Result<(), Box<Error + Send>> {
            println!("  version: {}", req.http_version());
            println!("  method: {:?}", req.method());
            println!("  scheme: {:?}", req.scheme());
            println!("  host: {:?}", req.host());
            println!("  path: {}", req.path());
            println!("  query_string: {:?}", req.query_string());
            println!("  remote_addr: {:?}", req.remote_addr());
            for &(k, ref v) in &req.headers().all() {
                println!("  hdr: {}={:?}", k, v);
            }
            Ok(())
        }
        fn after(
            &self,
            _req: &mut conduit::Request,
            res: Result<conduit::Response, Box<Error + Send>>,
        ) -> Result<conduit::Response, Box<Error + Send>> {
            res.map(|res| {
                println!("  <- {:?}", res.status);
                for (k, v) in &res.headers {
                    println!("  <- {} {:?}", k, v);
                }
                res
            })
        }
    }
}

/// Convenience function for getting the current server time in UTC.
pub fn now() -> time::Timespec {
    time::now_utc().to_timespec()
}

/// Convenience function for getting a time in RFC 3339 format.
///
/// Example: `2012-02-22T14:53:18Z`. Used for returning time values in JSON API responses.
pub fn encode_time(ts: time::Timespec) -> String {
    time::at_utc(ts).rfc3339().to_string()
}

/// Convenience function requiring that an environment variable is set.
///
/// Ensures that we've initialized the dotenv crate in order to read environment variables
/// from a *.env* file if present. Don't use this for optionally set environment variables.
///
/// # Panics
///
/// Panics if the environment variable with the name passed in as an argument is not defined
/// in the current environment.
pub fn env(s: &str) -> String {
    dotenv::dotenv().ok();
    ::std::env::var(s).unwrap_or_else(|_| panic!("must have `{}` defined", s))
}

sql_function!(lower, lower_t, (x: ::diesel::types::Text) -> ::diesel::types::Text);
