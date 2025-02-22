use axum::Json;
use axum::extract::{MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http::{Method, StatusCode, header};
use std::str::FromStr;

#[derive(Clone, Copy, Debug)]
pub enum StatusCodeConfig {
    /// Use the original response status code that the backend returned.
    Disabled,
    /// Use `200 OK` for all responses to cargo-relevant endpoints.
    AdjustAll,
    /// Use `200 OK` for all `2xx` responses to cargo-relevant endpoints, and
    /// the original status code for all other responses.
    AdjustSuccess,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to parse StatusCodeConfig")]
pub struct StatusCodeConfigError;

impl FromStr for StatusCodeConfig {
    type Err = StatusCodeConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "disabled" => Ok(Self::Disabled),
            "adjust-all" => Ok(Self::AdjustAll),
            "adjust-success" => Ok(Self::AdjustSuccess),
            _ => Err(StatusCodeConfigError),
        }
    }
}

/// Convert plain text errors into JSON errors and adjust status codes.
pub async fn middleware(
    State(config): State<StatusCodeConfig>,
    req: Request,
    next: Next,
) -> Response {
    let is_api_request = req.uri().path().starts_with("/api/");
    let is_cargo_endpoint = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| is_cargo_endpoint(req.method(), m.as_str()))
        .unwrap_or(false);

    let mut res = next.run(req).await;
    if is_api_request {
        res = ensure_json_errors(res).await;
    }
    if is_cargo_endpoint {
        // cargo until 1.34.0 expected crates.io to always return 200 OK for
        // all requests, even if they failed. If a different status code was
        // returned, cargo would show the raw JSON response to the user, instead
        // of a friendly error message.
        //
        // With cargo 1.34.0 this issue got resolved (see https://github.com/rust-lang/cargo/pull/6771),
        // for successful requests still only "200 OK" was expected and no other
        // 2xx status code. This will change with cargo 1.76.0 (see https://github.com/rust-lang/cargo/pull/13158),
        // but for backwards compatibility we still return "200 OK" for now for
        // all endpoints that are relevant for cargo.
        let adjust_status_code = matches!(
            (config, res.status().is_success()),
            (StatusCodeConfig::AdjustAll, _) | (StatusCodeConfig::AdjustSuccess, true)
        );
        if adjust_status_code {
            *res.status_mut() = StatusCode::OK;
        }
    }

    res
}

fn is_cargo_endpoint(method: &Method, path: &str) -> bool {
    const CARGO_ENDPOINTS: &[(Method, &str)] = &[
        (Method::PUT, "/api/v1/crates/new"),
        (Method::DELETE, "/api/v1/crates/{crate_id}/{version}/yank"),
        (Method::PUT, "/api/v1/crates/{crate_id}/{version}/unyank"),
        (Method::GET, "/api/v1/crates/{crate_id}/owners"),
        (Method::PUT, "/api/v1/crates/{crate_id}/owners"),
        (Method::DELETE, "/api/v1/crates/{crate_id}/owners"),
        (Method::GET, "/api/v1/crates"),
    ];

    CARGO_ENDPOINTS
        .iter()
        .any(|(m, p)| m == method && p == &path)
}

/// Convert plain text errors into JSON errors.
///
/// The built-in extractors in [axum] return plain text errors, but our API
/// contract promises JSON errors. This middleware converts such plain text
/// errors into corresponding JSON errors, allowing us to use the [axum]
/// extractors without having to care about the error responses.
async fn ensure_json_errors(res: Response) -> Response {
    let status = res.status();
    if !status.is_client_error() && !status.is_server_error() {
        return res;
    }

    let content_type = res.headers().get("content-type");
    if !matches!(content_type, Some(content_type) if content_type == "text/plain; charset=utf-8") {
        return res;
    }

    convert_to_json_response(res).await.unwrap_or_else(|error| {
        error!(%error, "Failed to convert response to JSON");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })
}

async fn convert_to_json_response(res: Response) -> anyhow::Result<Response> {
    let (mut parts, body) = res.into_parts();

    // The `Json` struct is somehow not able to override these headers of the
    // `Parts` struct, so we remove them here to avoid the conflict.
    parts.headers.remove(header::CONTENT_TYPE);
    parts.headers.remove(header::CONTENT_LENGTH);

    let bytes = axum::body::to_bytes(body, 1_000_000).await?;
    let text = std::str::from_utf8(&bytes)?;

    let json = serde_json::json!({ "errors": [{ "detail": text }] });

    Ok((parts, Json(json)).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::middleware::from_fn_with_state;
    use axum::routing::{get, put};
    use bytes::Bytes;
    use http::response::Parts;
    use http::{Request, StatusCode};
    use insta::assert_debug_snapshot;
    use tower::ServiceExt;

    fn build_app() -> Router {
        let okay = get(async || "Everything is okay");
        let teapot = get(async || (StatusCode::IM_A_TEAPOT, "I'm a teapot"));
        let internal = get(async || (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"));

        Router::new()
            .route("/api/ok", okay.clone())
            .route("/api/teapot", teapot.clone())
            .route("/teapot", teapot)
            .route("/api/500", internal.clone())
            .route("/500", internal)
            .route("/api/v1/crates/new", put(async || StatusCode::CREATED))
            .route(
                "/api/v1/crates/{crate_id}/owners",
                get(async || StatusCode::INTERNAL_SERVER_ERROR),
            )
            .layer(from_fn_with_state(StatusCodeConfig::AdjustAll, middleware))
    }

    async fn request(path: &str) -> anyhow::Result<(Parts, Bytes)> {
        request_inner(Method::GET, path).await
    }

    async fn put_request(path: &str) -> anyhow::Result<(Parts, Bytes)> {
        request_inner(Method::PUT, path).await
    }

    async fn request_inner(method: Method, path: &str) -> anyhow::Result<(Parts, Bytes)> {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())?;
        let response = build_app().oneshot(request).await?;
        let (parts, body) = response.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX).await?;
        Ok((parts, bytes))
    }

    /// Check that successful text responses are **not** converted to JSON even
    /// for `/api/` requests.
    #[tokio::test]
    async fn test_success_responses() {
        let (parts, bytes) = request("/api/ok").await.unwrap();
        assert_eq!(parts.status, StatusCode::OK);
        assert_debug_snapshot!(parts.headers, @r#"
        {
            "content-type": "text/plain; charset=utf-8",
            "content-length": "18",
        }
        "#);
        assert_debug_snapshot!(bytes, @r#"b"Everything is okay""#);
    }

    /// Check that 4xx text responses **are** converted to JSON, but only
    /// for `/api/` requests.
    #[tokio::test]
    async fn test_client_errors() {
        let (parts, bytes) = request("/api/teapot").await.unwrap();
        assert_eq!(parts.status, StatusCode::IM_A_TEAPOT);
        assert_debug_snapshot!(parts.headers, @r#"
        {
            "content-type": "application/json",
            "content-length": "38",
        }
        "#);
        assert_debug_snapshot!(bytes, @r#"b"{\"errors\":[{\"detail\":\"I'm a teapot\"}]}""#);

        let (parts, bytes) = request("/teapot").await.unwrap();
        assert_eq!(parts.status, StatusCode::IM_A_TEAPOT);
        assert_debug_snapshot!(parts.headers, @r#"
        {
            "content-type": "text/plain; charset=utf-8",
            "content-length": "12",
        }
        "#);
        assert_debug_snapshot!(bytes, @r#"b"I'm a teapot""#);
    }

    /// Check that 5xx text responses **are** converted to JSON, but only
    /// for `/api/` requests.
    #[tokio::test]
    async fn test_server_errors() {
        let (parts, bytes) = request("/api/500").await.unwrap();
        assert_eq!(parts.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_debug_snapshot!(parts.headers, @r#"
        {
            "content-type": "application/json",
            "content-length": "47",
        }
        "#);
        assert_debug_snapshot!(bytes, @r#"b"{\"errors\":[{\"detail\":\"Internal Server Error\"}]}""#);

        let (parts, bytes) = request("/500").await.unwrap();
        assert_eq!(parts.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_debug_snapshot!(parts.headers, @r#"
        {
            "content-type": "text/plain; charset=utf-8",
            "content-length": "21",
        }
        "#);
        assert_debug_snapshot!(bytes, @r#"b"Internal Server Error""#);
    }

    #[tokio::test]
    async fn test_cargo_endpoint_status() {
        let (parts, _bytes) = put_request("/api/v1/crates/new").await.unwrap();
        assert_eq!(parts.status, StatusCode::OK);

        let (parts, _bytes) = request("/api/v1/crates/foo/owners").await.unwrap();
        assert_eq!(parts.status, StatusCode::OK);
    }
}
