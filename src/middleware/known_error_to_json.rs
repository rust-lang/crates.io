//! Converts known error types into friendly JSON errors
//!
//! Some similar logic exists in `crate::util::errors::AppError::try_convert()`. That low-level
//! handling needs to remain in place, because some endpoint logic relys on detecting those
//! normalized errors. Errors produced by the router cannot be seen by endpoints, so the conversion
//! can be deferred until here.

use super::prelude::*;
use crate::util::errors::NotFound;

use conduit_router::RouterError;

#[derive(Default)]
pub struct KnownErrorToJson;

impl Middleware for KnownErrorToJson {
    fn after(&self, _: &mut dyn RequestExt, res: AfterResult) -> AfterResult {
        res.or_else(|e| {
            if e.downcast_ref::<RouterError>().is_some() {
                return Ok(NotFound.into());
            }

            Err(e)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::KnownErrorToJson;

    use conduit::{Body, Handler, Method, StatusCode};
    use conduit_middleware::MiddlewareBuilder;
    use conduit_router::RouteBuilder;
    use conduit_test::MockRequest;

    #[test]
    fn router_errors_become_not_found_response() {
        let route_builder = RouteBuilder::new();
        let mut middleware = MiddlewareBuilder::new(route_builder);
        middleware.add(KnownErrorToJson);

        let mut req = MockRequest::new(Method::GET, "/");
        let (parts, body) = middleware.call(&mut req).unwrap().into_parts();
        assert_eq!(parts.status, StatusCode::NOT_FOUND);
        assert!(matches!(
            body,
            Body::Owned(vec) if vec == br#"{"errors":[{"detail":"Not Found"}]}"#
        ));
    }
}
