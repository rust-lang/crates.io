use std::any::AnyRefExt;
use std::fmt::Show;

use conduit_middleware;
use conduit::Request;
use conduit_cookie::RequestSession;

use app::RequestApp;
use super::User;

pub struct Middleware;

impl conduit_middleware::Middleware for Middleware {
    fn before(&self, req: &mut Request) -> Result<(), Box<Show>> {
        let id = match req.session().find_equiv(&"user_id")
                          .and_then(|s| from_str(s.as_slice())) {
            Some(id) => id,
            None => return Ok(()),
        };
        let user = match User::find(req.app(), id) {
            Some(user) => user,
            None => return Ok(()),
        };

        req.mut_extensions().insert("crates.io.user", box user);
        Ok(())
    }
}

pub trait RequestUser<'a> {
    fn user(self) -> Option<&'a User>;
}

impl<'a> RequestUser<'a> for &'a Request {
    fn user(self) -> Option<&'a User> {
        self.extensions().find_equiv(&"crates.io.user").and_then(|r| {
            r.as_ref::<User>()
        })
    }
}
