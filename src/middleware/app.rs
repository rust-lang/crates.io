use super::prelude::*;

use crate::App;
use std::sync::Arc;

/// Middleware that injects the `App` instance into the `Request` extensions
pub struct AppMiddleware {
    app: Arc<App>,
}

impl AppMiddleware {
    pub fn new(app: Arc<App>) -> AppMiddleware {
        AppMiddleware { app }
    }
}

impl Middleware for AppMiddleware {
    fn before(&self, req: &mut dyn RequestExt) -> BeforeResult {
        req.mut_extensions().insert(Arc::clone(&self.app));
        Ok(())
    }

    fn after(&self, req: &mut dyn RequestExt, res: AfterResult) -> AfterResult {
        req.mut_extensions().remove::<Arc<App>>().unwrap();
        res
    }
}

/// Adds an `app()` method to the `Request` type returning the global `App` instance
pub trait RequestApp {
    fn app(&self) -> &Arc<App>;
}

impl<T: RequestExt + ?Sized> RequestApp for T {
    fn app(&self) -> &Arc<App> {
        self.extensions().get::<Arc<App>>().expect("Missing app")
    }
}
