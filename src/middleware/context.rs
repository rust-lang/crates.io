use crate::controllers::util::RequestPartsExt;
use crate::server::ServerContext;

/// Adds a `server_context()` method to request types.
pub trait RequestContext {
    /// Returns the server context for this request.
    fn server_context(&self) -> &ServerContext;
}

impl<T: RequestPartsExt> RequestContext for T {
    fn server_context(&self) -> &ServerContext {
        self.extensions()
            .get::<ServerContext>()
            .expect("Missing server context")
    }
}
