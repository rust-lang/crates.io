//! Middleware that captures the `user_id` from the signed session cookie
//!
//! Due to lifetimes it is not possible to call `mut_extensions()` while a reference obtained from
//! `extensions()` is still live.  The call to `conduit_cookie::RequestSession::session` needs
//! mutable access to the request and its extensions, and there is no read-only alternative in
//! `conduit_cookie` to access the session cookie.  This means that it is not possible to access
//! the session cookie while holding onto a database connection (which is obtained from the
//! `AppMiddleware` via `extensions()`).
//!
//! This is particularly problematic for the user authentication code.  When an API token is used
//! for authentication, the datbase must be queried to obtain the `user_id`, so endpoint code must
//! obtain and pass in a database connection.  Because of that connection, it is no longer possible
//! to use or pass around the `&mut dyn Request` that it was derived from and it is not possible
//! to access the session cookie.  In order to support authentication via session cookies and API
//! tokens via the same code path, the `user_id` is extracted from the session cookie and stored in
//! a `TrustedUserId` that can be read from while a connection reference is live.

use super::prelude::*;

use conduit_cookie::RequestSession;

/// A trusted user_id extracted from a signed cookie or added to the request by the test harness
#[derive(Clone, Copy, Debug)]
pub struct TrustedUserId(pub i32);

/// Middleware that captures the `user_id` from the signed session cookie
pub(super) struct CaptureUserIdFromCookie;

impl Middleware for CaptureUserIdFromCookie {
    fn before(&self, req: &mut dyn Request) -> Result<()> {
        if let Some(id) = req.session().get("user_id").and_then(|s| s.parse().ok()) {
            req.mut_extensions().insert(TrustedUserId(id));
        }

        Ok(())
    }
}
