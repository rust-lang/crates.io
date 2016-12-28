use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use conduit::{Request, Response};
use conduit_middleware::Middleware;
use git2;
use oauth2;
use r2d2;
use s3;
use curl::easy::Easy;

use {db, Config};

/// The `App` struct holds the main components of the application like
/// the database connection pool and configurations
pub struct App {
    /// The database connection pool
    pub database: db::Pool,

    /// The GitHub OAuth2 configuration
    pub github: oauth2::Config,
    pub bucket: s3::Bucket,
    pub s3_proxy: Option<String>,
    pub session_key: String,
    pub git_repo: Mutex<git2::Repository>,
    pub git_repo_checkout: PathBuf,

    /// The server configuration
    pub config: Config,
}

/// The `AppMiddleware` injects an `App` instance into the `Request` extensions
pub struct AppMiddleware {
    app: Arc<App>
}

impl App {
    pub fn new(config: &Config) -> App {
        let mut github = oauth2::Config::new(
            &config.gh_client_id,
            &config.gh_client_secret,
            "https://github.com/login/oauth/authorize",
            "https://github.com/login/oauth/access_token",
        );

        github.scopes.push(String::from("read:org"));

        let db_config = r2d2::Config::builder()
            .pool_size(if config.env == ::Env::Production {10} else {1})
            .helper_threads(if config.env == ::Env::Production {3} else {1})
            .build();

        let repo = git2::Repository::open(&config.git_repo_checkout).unwrap();
        return App {
            database: db::pool(&config.db_url, db_config),
            github: github,
            bucket: s3::Bucket::new(config.s3_bucket.clone(),
                                    config.s3_region.clone(),
                                    config.s3_access_key.clone(),
                                    config.s3_secret_key.clone(),
                                    config.api_protocol()),
            s3_proxy: config.s3_proxy.clone(),
            session_key: config.session_key.clone(),
            git_repo: Mutex::new(repo),
            git_repo_checkout: config.git_repo_checkout.clone(),
            config: config.clone(),
        };
    }

    pub fn handle(&self) -> Easy {
        let mut handle = Easy::new();
        if let Some(ref proxy) = self.s3_proxy {
            handle.proxy(proxy).unwrap();
        }
        return handle
    }
}

impl AppMiddleware {
    pub fn new(app: Arc<App>) -> AppMiddleware {
        AppMiddleware { app: app }
    }
}

impl Middleware for AppMiddleware {
    fn before(&self, req: &mut Request) -> Result<(), Box<Error+Send>> {
        req.mut_extensions().insert(self.app.clone());
        Ok(())
    }

    fn after(&self, req: &mut Request, res: Result<Response, Box<Error+Send>>)
             -> Result<Response, Box<Error+Send>> {
        req.mut_extensions().pop::<Arc<App>>().unwrap();
        res
    }
}

/// Adds an `app()` method to the `Request` type returning the global `App` instance
pub trait RequestApp {
    fn app(&self) -> &Arc<App>;
}

impl<'a> RequestApp for Request + 'a {
    fn app(&self) -> &Arc<App> {
        self.extensions().find::<Arc<App>>()
            .expect("Missing app")
    }
}
