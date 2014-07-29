use std::any::AnyRefExt;
use std::fmt::Show;
use std::os;
use std::sync::Mutex;

use conduit::Request;
use conduit_middleware::Middleware;
use oauth2;
use pg::pool::{PostgresConnectionPool, PooledPostgresConnection};
use s3;

use db;

use std::sync::Arc;

pub struct App {
    db: PostgresConnectionPool,
    pub github: oauth2::Config,
    pub bucket: s3::Bucket,
    pub session_key: String,
    pub git_repo_bare: Path,
    pub git_repo_checkout: Mutex<Path>,
}

pub struct AppMiddleware {
    app: Arc<App>
}

impl App {
    pub fn new() -> App {
        let pool = db::pool();
        db::setup(&*pool.get_connection());
        let github = oauth2::Config::new(
            env("GH_CLIENT_ID").as_slice(),
            env("GH_CLIENT_SECRET").as_slice(),
            "https://github.com/login/oauth/authorize",
            "https://github.com/login/oauth/access_token",
        );

        return App {
            db: db::pool(),
            github: github,
            bucket: s3::Bucket::new(env("S3_BUCKET"),
                                    env("S3_ACCESS_KEY"),
                                    env("S3_SECRET_KEY")),
            session_key: env("SESSION_KEY"),
            git_repo_bare: Path::new(env("GIT_REPO_BARE")),
            git_repo_checkout: Mutex::new(Path::new(env("GIT_REPO_CHECKOUT"))),
        };

        fn env(s: &str) -> String {
            match os::getenv(s) {
                Some(s) => s,
                None => fail!("must have `{}` defined", s),
            }
        }
    }

    pub fn db(&self) -> PooledPostgresConnection {
        self.db.get_connection()
    }
}

impl AppMiddleware {
    pub fn new(app: App) -> AppMiddleware {
        AppMiddleware { app: Arc::new(app) }
    }
}

impl Middleware for AppMiddleware {
    fn before(&self, req: &mut Request) -> Result<(), Box<Show>> {
        req.mut_extensions().insert("crates.io.app", box self.app.clone());
        Ok(())
    }
}

pub trait RequestApp<'a> {
    fn app(self) -> &'a Arc<App>;
}

impl<'a> RequestApp<'a> for &'a Request {
    fn app(self) -> &'a Arc<App> {
        self.extensions().find(&"crates.io.app")
            .and_then(|a| a.downcast_ref::<Arc<App>>())
            .expect("Missing app")
    }
}
