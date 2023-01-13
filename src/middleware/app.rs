use axum::middleware::Next;
use axum::response::Response;
use http::Request;

use crate::app::AppState;
use crate::controllers::util::RequestPartsExt;

/// `axum` middleware that injects the `AppState` instance into the `Request` extensions.
pub async fn add_app_state_extension<B>(
    app_state: AppState,
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

impl<T: RequestPartsExt> RequestApp for T {
    fn app(&self) -> &AppState {
        self.extensions().get::<AppState>().expect("Missing app")
    }
}
