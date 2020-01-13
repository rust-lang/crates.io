use super::prelude::*;

use crate::App;
use std::sync::Arc;

/// Middleware that injects the `App` instance into the `Request` extensions
// Can't derive Debug because `App` can't.
#[allow(missing_debug_implementations)]
pub struct AppMiddleware {
    app: Arc<App>,
}

impl AppMiddleware {
    pub fn new(app: Arc<App>) -> AppMiddleware {
        AppMiddleware { app }
    }
}

impl Middleware for AppMiddleware {
    fn before(&self, req: &mut dyn Request) -> Result<()> {
        req.mut_extensions().insert(Arc::clone(&self.app));
        Ok(())
    }

    fn after(&self, req: &mut dyn Request, res: Result<Response>) -> Result<Response> {
        req.mut_extensions().pop::<Arc<App>>().unwrap();
        res
    }
}

/// Adds an `app()` method to the `Request` type returning the global `App` instance
pub trait RequestApp {
    fn app(&self) -> &Arc<App>;
}

impl<T: Request + ?Sized> RequestApp for T {
    fn app(&self) -> &Arc<App> {
        self.extensions().find::<Arc<App>>().expect("Missing app")
    }
}
