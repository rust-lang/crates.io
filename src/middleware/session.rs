use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::SignedCookieJar;
use conduit_cookie::SessionMiddleware;
use cookie::time::Duration;
use cookie::{Cookie, SameSite};
use http::Request;
use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

static COOKIE_NAME: &str = "cargo_session";
static MAX_AGE_DAYS: i64 = 90;

pub async fn attach_session<B>(
    jar: SignedCookieJar,
    mut req: Request<B>,
    next: Next<B>,
) -> Response {
    // Decode session cookie
    let data = jar
        .get(COOKIE_NAME)
        .map(SessionMiddleware::decode)
        .unwrap_or_default();

    // Save decoded session data in request extension,
    // and keep an `Arc` clone for later
    let session = Arc::new(RwLock::new(Session::new(data)));
    req.extensions_mut().insert(session.clone());

    // Process the request
    let response = next.run(req).await;

    // Check if the session data was mutated
    let session = session.read().unwrap();
    if session.dirty {
        // Return response with additional `Set-Cookie` header
        let encoded = SessionMiddleware::encode(&session.data);
        let cookie = Cookie::build(COOKIE_NAME, encoded)
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Strict)
            .max_age(Duration::days(MAX_AGE_DAYS))
            .path("/")
            .finish();

        (jar.add(cookie), response).into_response()
    } else {
        response
    }
}

/// Request extension holding the session data
struct Session {
    data: HashMap<String, String>,
    dirty: bool,
}

impl Session {
    fn new(data: HashMap<String, String>) -> Self {
        Self { data, dirty: false }
    }
}

pub trait RequestSession {
    fn session_get(&self, key: &str) -> Option<String>;
    fn session_insert(&mut self, key: String, value: String) -> Option<String>;
    fn session_remove(&mut self, key: &str) -> Option<String>;
}

impl<T: conduit::RequestExt + ?Sized> RequestSession for T {
    fn session_get(&self, key: &str) -> Option<String> {
        let session = self
            .extensions()
            .get::<Arc<RwLock<Session>>>()
            .expect("missing cookie session")
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        session.data.get(key).cloned()
    }

    fn session_insert(&mut self, key: String, value: String) -> Option<String> {
        let mut session = self
            .mut_extensions()
            .get_mut::<Arc<RwLock<Session>>>()
            .expect("missing cookie session")
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        session.dirty = true;
        session.data.insert(key, value)
    }

    fn session_remove(&mut self, key: &str) -> Option<String> {
        let mut session = self
            .mut_extensions()
            .get_mut::<Arc<RwLock<Session>>>()
            .expect("missing cookie session")
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        session.dirty = true;
        session.data.remove(key)
    }
}
