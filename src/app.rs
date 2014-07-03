use oauth2;
use conduit::Request;
use conduit_middleware::Middleware;
use pg::pool::{PostgresConnectionPool, PooledPostgresConnection};
use std::any::AnyRefExt;
use std::fmt::Show;

use db;

use std::sync::Arc;

pub struct App {
    db: PostgresConnectionPool,
    pub github: oauth2::Config,
}

pub struct AppMiddleware {
    app: Arc<App>
}

impl App {
    pub fn new() -> App {
        let pool = db::pool();
        db::setup(&*pool.get_connection());
        let github = oauth2::Config::new(
            "89b6afdeaa6c6c7506ec",
            "7a4908a38c75dd12bce36931ad2dbdd951ce228b",
            "https://github.com/login/oauth/authorize",
            "https://github.com/login/oauth/access_token",
        );

        App {
            db: db::pool(),
            github: github,
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
