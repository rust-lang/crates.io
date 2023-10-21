use crate::app::AppState;
use crate::controllers::util::RequestPartsExt;

/// Adds an `app()` method to the `Request` type returning the global `App` instance
pub trait RequestApp {
    fn app(&self) -> &AppState;
}

impl<T: RequestPartsExt> RequestApp for T {
    fn app(&self) -> &AppState {
        self.extensions().get::<AppState>().expect("Missing app")
    }
}
