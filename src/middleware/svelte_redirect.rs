//! Redirect `/svelte/*` paths to their canonical un-prefixed URL.
//!
//! While the SvelteKit frontend was being developed, it was served at
//! `/svelte/`. Now that Svelte is the default frontend at `/`, this
//! middleware 307-redirects any leftover `/svelte/...` URL (bookmarks,
//! external links, in-flight HTML pages cached by browsers/CDNs) back
//! to the canonical un-prefixed location.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};

const PREFIX: &str = "/svelte";

pub async fn redirect(request: Request, next: Next) -> Response {
    let Some(target) = redirect_target(request.uri().path(), request.uri().query()) else {
        return next.run(request).await;
    };
    Redirect::temporary(&target).into_response()
}

/// Computes the redirect target for a request to a `/svelte/...` path.
///
/// Returns [`None`] if the request should not be redirected. The result
/// preserves the query string, if any.
fn redirect_target(path: &str, query: Option<&str>) -> Option<String> {
    let stripped = path.strip_prefix(PREFIX)?;

    // Only match `/svelte` exactly or `/svelte/...`. Avoid matching paths
    // that just happen to share the prefix (e.g. `/sveltefoo`).
    if !(stripped.is_empty() || stripped.starts_with('/')) {
        return None;
    }

    let new_path = if stripped.is_empty() { "/" } else { stripped };
    Some(match query {
        Some(query) => format!("{new_path}?{query}"),
        None => new_path.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use crate::middleware::svelte_redirect::redirect_target;

    #[test]
    fn test_redirect_target() {
        const CASES: &[(&str, Option<&str>, Option<&str>)] = &[
            // Bare prefix
            ("/svelte", None, Some("/")),
            ("/svelte/", None, Some("/")),
            // Deep paths
            ("/svelte/crates/tokio", None, Some("/crates/tokio")),
            ("/svelte/crates/tokio/", None, Some("/crates/tokio/")),
            // Query string preserved
            (
                "/svelte/crates/tokio",
                Some("foo=1"),
                Some("/crates/tokio?foo=1"),
            ),
            ("/svelte", Some("x=y"), Some("/?x=y")),
            // Non-matching paths
            ("/", None, None),
            ("/crates/tokio", None, None),
            ("/sveltefoo", None, None),
            ("/sveltey/x", None, None),
        ];

        for (path, query, expected) in CASES.iter().copied() {
            assert_eq!(redirect_target(path, query).as_deref(), expected);
        }
    }
}
