use std::fmt::Show;
use std::sync::{Arc, Mutex};

use r2d2;
use conduit::Request;
use conduit_middleware::Middleware;
use oauth2;
use s3;

use {db, Config};

pub struct App {
    pub database: db::Pool,
    pub github: oauth2::Config,
    pub bucket: s3::Bucket,
    pub s3_proxy: Option<String>,
    pub session_key: String,
    pub git_repo_bare: Path,
    pub git_repo_checkout: Mutex<Path>,
    pub env: ::Environment,
}

pub struct AppMiddleware {
    app: Arc<App>
}

impl App {
    pub fn new(config: &Config) -> App {
        let github = oauth2::Config::new(
            config.gh_client_id.as_slice(),
            config.gh_client_secret.as_slice(),
            "https://github.com/login/oauth/authorize",
            "https://github.com/login/oauth/access_token",
        );

        let db_config = r2d2::Config {
            pool_size: if config.env == ::Production {10} else {1},
            helper_tasks: if config.env == ::Production {3} else {1},
            test_on_check_out: false,
        };

        return App {
            database: db::pool(config.db_url.as_slice(), db_config),
            github: github,
            bucket: s3::Bucket::new(config.s3_bucket.clone(),
                                    config.s3_access_key.clone(),
                                    config.s3_secret_key.clone(),
                                    if config.env == ::Test {"http"} else {"https"}),
            s3_proxy: config.s3_proxy.clone(),
            session_key: config.session_key.clone(),
            git_repo_bare: config.git_repo_bare.clone(),
            git_repo_checkout: Mutex::new(config.git_repo_checkout.clone()),
            env: config.env,
        };
    }

    pub fn db_setup(&self) {
        db::setup(&*self.database.get().unwrap())
    }
}

impl AppMiddleware {
    pub fn new(app: App) -> AppMiddleware {
        AppMiddleware { app: Arc::new(app) }
    }
}

impl Middleware for AppMiddleware {
    fn before(&self, req: &mut Request) -> Result<(), Box<Show + 'static>> {
        req.mut_extensions().insert(self.app.clone());
        Ok(())
    }
}

pub trait RequestApp<'a> {
    fn app(self) -> &'a Arc<App>;
}

impl<'a> RequestApp<'a> for &'a Request + 'a {
    fn app(self) -> &'a Arc<App> {
        self.extensions().find::<Arc<App>>()
            .expect("Missing app")
    }
}
