use super::prelude::*;
use axum::extract::State;
use axum::middleware::Next;
use axum::response::Response;
use http::Request;

use crate::app::AppState;

/// Middleware that injects the `App` instance into the `Request` extensions
pub struct AppMiddleware {
    app: AppState,
}

impl AppMiddleware {
    pub fn new(app: AppState) -> AppMiddleware {
        AppMiddleware { app }
    }
}

impl Middleware for AppMiddleware {
    fn before(&self, req: &mut dyn RequestExt) -> BeforeResult {
        req.mut_extensions().insert(self.app.clone());
        Ok(())
    }
}

/// `axum` middleware that injects the `AppState` instance into the `Request` extensions.
pub async fn add_app_state_extension<B>(
    State(app_state): State<AppState>,
    mut request: Request<B>,
    next: Next<B>,
) -> Response {
    request.extensions_mut().insert(app_state);

    next.run(request).await
}

/// Adds an `app()` method to the `Request` type returning the global `App` instance
pub trait RequestApp {
    fn app(&self) -> &AppState;
}

impl<T: RequestExt + ?Sized> RequestApp for T {
    fn app(&self) -> &AppState {
        self.extensions().get::<AppState>().expect("Missing app")
    }
}
