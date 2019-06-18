use std::error::Error;
use std::sync::Arc;

use conduit::{Handler, Request, Response};
use conduit_router::{RequestParams, RouteBuilder};

use crate::controllers::*;
use crate::util::errors::{std_error, CargoError, CargoResult, NotFound};
use crate::util::RequestProxy;
use crate::{App, Env};

pub fn build_router(app: &App) -> R404 {
    let mut api_router = RouteBuilder::new();

    // Route used by both `cargo search` and the frontend
    api_router.get("/crates", C(krate::search::search));

    // Routes used by `cargo`
    api_router.put("/crates/new", C(krate::publish::publish));
    api_router.get("/crates/:crate_id/owners", C(krate::owners::owners));
    api_router.put("/crates/:crate_id/owners", C(krate::owners::add_owners));
    api_router.delete("/crates/:crate_id/owners", C(krate::owners::remove_owners));
    api_router.delete("/crates/:crate_id/:version/yank", C(version::yank::yank));
    api_router.put(
        "/crates/:crate_id/:version/unyank",
        C(version::yank::unyank),
    );
    api_router.get(
        "/crates/:crate_id/:version/download",
        C(version::downloads::download),
    );

    // Routes that appear to be unused
    api_router.get("/versions", C(version::deprecated::index));
    api_router.get("/versions/:version_id", C(version::deprecated::show_by_id));

    // Routes used by the frontend
    api_router.get("/crates/:crate_id", C(krate::metadata::show));
    api_router.get("/crates/:crate_id/:version", C(version::metadata::show));
    api_router.get(
        "/crates/:crate_id/:version/readme",
        C(krate::metadata::readme),
    );
    api_router.get(
        "/crates/:crate_id/:version/dependencies",
        C(version::metadata::dependencies),
    );
    api_router.get(
        "/crates/:crate_id/:version/downloads",
        C(version::downloads::downloads),
    );
    api_router.get(
        "/crates/:crate_id/:version/authors",
        C(version::metadata::authors),
    );
    api_router.get(
        "/crates/:crate_id/downloads",
        C(krate::downloads::downloads),
    );
    api_router.get("/crates/:crate_id/versions", C(krate::metadata::versions));
    api_router.put("/crates/:crate_id/follow", C(krate::follow::follow));
    api_router.delete("/crates/:crate_id/follow", C(krate::follow::unfollow));
    api_router.get("/crates/:crate_id/following", C(krate::follow::following));
    api_router.get("/crates/:crate_id/owner_team", C(krate::owners::owner_team));
    api_router.get("/crates/:crate_id/owner_user", C(krate::owners::owner_user));
    api_router.get(
        "/crates/:crate_id/reverse_dependencies",
        C(krate::metadata::reverse_dependencies),
    );
    api_router.get("/keywords", C(keyword::index));
    api_router.get("/keywords/:keyword_id", C(keyword::show));
    api_router.get("/categories", C(category::index));
    api_router.get("/categories/:category_id", C(category::show));
    api_router.get("/category_slugs", C(category::slugs));
    api_router.get("/users/:user_id", C(user::other::show));
    api_router.put("/users/:user_id", C(user::me::update_user));
    api_router.get("/users/:user_id/stats", C(user::other::stats));
    api_router.get("/teams/:team_id", C(team::show_team));
    api_router.get("/users/:user_id/favorited", C(user::me::favorited));
    api_router.put("/users/:user_id/favorite", C(user::me::favorite));
    api_router.delete("/users/:user_id/favorite", C(user::me::unfavorite));
    api_router.get(
        "/users/:user_id/favorite_users",
        C(user::me::favorite_users),
    );
    api_router.get("/me", C(user::me::me));
    api_router.get("/me/updates", C(user::me::updates));
    api_router.get("/me/tokens", C(token::list));
    api_router.put("/me/tokens", C(token::new));
    api_router.delete("/me/tokens/:id", C(token::revoke));
    api_router.get(
        "/me/crate_owner_invitations",
        C(crate_owner_invitation::list),
    );
    api_router.put(
        "/me/crate_owner_invitations/:crate_id",
        C(crate_owner_invitation::handle_invite),
    );
    api_router.get("/summary", C(krate::metadata::summary));
    api_router.put("/confirm/:email_token", C(user::me::confirm_user_email));
    api_router.put(
        "/users/:user_id/resend",
        C(user::me::regenerate_token_and_send),
    );
    api_router.get("/site_metadata", C(site_metadata::show_deployed_sha));
    let api_router = Arc::new(R404(api_router));

    let mut router = RouteBuilder::new();

    // Mount the router under the /api/v1 path so we're at least somewhat at the
    // liberty to change things in the future!
    router.get("/api/v1/*path", R(Arc::clone(&api_router)));
    router.put("/api/v1/*path", R(Arc::clone(&api_router)));
    router.post("/api/v1/*path", R(Arc::clone(&api_router)));
    router.head("/api/v1/*path", R(Arc::clone(&api_router)));
    router.delete("/api/v1/*path", R(api_router));

    router.get("/authorize_url", C(user::session::github_authorize));
    router.get("/authorize", C(user::session::github_access_token));
    router.delete("/logout", C(user::session::logout));

    // Only serve the local checkout of the git index in development mode.
    // In production, for crates.io, cargo gets the index from
    // https://github.com/rust-lang/crates.io-index directly.
    if app.config.env == Env::Development {
        let s = conduit_git_http_backend::Serve(app.git_repo_checkout.clone());
        let s = Arc::new(s);
        router.get("/git/index/*path", R(Arc::clone(&s)));
        router.post("/git/index/*path", R(s));
    }

    R404(router)
}

struct C(pub fn(&mut dyn Request) -> CargoResult<Response>);

impl Handler for C {
    fn call(&self, req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        let C(f) = *self;
        match f(req) {
            Ok(resp) => Ok(resp),
            Err(e) => match e.response() {
                Some(response) => Ok(response),
                None => Err(std_error(e)),
            },
        }
    }
}

struct R<H>(pub Arc<H>);

impl<H: Handler> Handler for R<H> {
    fn call(&self, req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        let path = req.params()["path"].to_string();
        let R(ref sub_router) = *self;
        sub_router.call(&mut RequestProxy {
            other: req,
            path: Some(&path),
            method: None,
        })
    }
}

// Can't derive Debug because of RouteBuilder.
#[allow(missing_debug_implementations)]
pub struct R404(pub RouteBuilder);

impl Handler for R404 {
    fn call(&self, req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        let R404(ref router) = *self;
        match router.recognize(&req.method(), req.path()) {
            Ok(m) => {
                req.mut_extensions().insert(m.params.clone());
                m.handler.call(req)
            }
            Err(..) => Ok(NotFound.response().unwrap()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::errors::{bad_request, human, internal, NotFound, Unauthorized};

    use conduit_test::MockRequest;
    use diesel::result::Error as DieselError;

    fn err<E: CargoError>(err: E) -> CargoResult<Response> {
        Err(Box::new(err))
    }

    #[test]
    fn http_error_responses() {
        let mut req = MockRequest::new(::conduit::Method::Get, "/");

        // Types for handling common error status codes
        assert_eq!(
            C(|_| Err(bad_request(""))).call(&mut req).unwrap().status.0,
            400
        );
        assert_eq!(
            C(|_| err(Unauthorized)).call(&mut req).unwrap().status.0,
            403
        );
        assert_eq!(
            C(|_| Err(DieselError::NotFound.into()))
                .call(&mut req)
                .unwrap()
                .status
                .0,
            404
        );
        assert_eq!(C(|_| err(NotFound)).call(&mut req).unwrap().status.0, 404);

        // Human errors are returned as 200 so that cargo displays this nicely on the command line
        assert_eq!(C(|_| Err(human(""))).call(&mut req).unwrap().status.0, 200);

        // All other error types are propogated up the middleware, eventually becoming status 500
        assert!(C(|_| Err(internal(""))).call(&mut req).is_err());
        assert!(C(|_| err(::serde_json::Error::syntax(
            ::serde_json::error::ErrorCode::ExpectedColon,
            0,
            0
        )))
        .call(&mut req)
        .is_err());
        assert!(
            C(|_| err(::std::io::Error::new(::std::io::ErrorKind::Other, "")))
                .call(&mut req)
                .is_err()
        );
    }
}
