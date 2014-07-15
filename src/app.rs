use std::any::AnyRefExt;
use std::fmt::Show;
use std::os;

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
    fn app(self) -> &'a App;
}

impl<'a> RequestApp<'a> for &'a mut Request {
    fn app(self) -> &'a App {
        &**self.extensions().find(&"crates.io.app")
               .and_then(|a| a.as_ref::<Arc<App>>())
               .expect("Missing app")
    }
}
