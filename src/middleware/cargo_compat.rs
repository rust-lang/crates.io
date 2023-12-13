use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use http::{header, StatusCode};

/// Convert plain text errors into JSON errors.
pub async fn middleware(req: Request, next: Next) -> Response {
    let is_api_request = req.uri().path().starts_with("/api/");

    let mut res = next.run(req).await;
    if is_api_request {
        res = ensure_json_errors(res).await;
    }

    res
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
    use axum::body::Body;
    use axum::middleware::from_fn;
    use axum::routing::get;
    use axum::Router;
    use bytes::Bytes;
    use http::response::Parts;
    use http::{Request, StatusCode};
    use insta::assert_debug_snapshot;
    use tower::ServiceExt;

    fn build_app() -> Router {
        let okay = get(|| async { "Everything is okay" });
        let teapot = get(|| async { (StatusCode::IM_A_TEAPOT, "I'm a teapot") });
        let internal =
            get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error") });

        Router::new()
            .route("/api/ok", okay.clone())
            .route("/api/teapot", teapot.clone())
            .route("/teapot", teapot)
            .route("/api/500", internal.clone())
            .route("/500", internal)
            .layer(from_fn(middleware))
    }

    async fn request(path: &str) -> anyhow::Result<(Parts, Bytes)> {
        let request = Request::get(path).body(Body::empty())?;
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
        assert_debug_snapshot!(parts.headers, @r###"
        {
            "content-type": "text/plain; charset=utf-8",
            "content-length": "18",
        }
        "###);
        assert_debug_snapshot!(bytes, @r###"b"Everything is okay""###);
    }

    /// Check that 4xx text responses **are** converted to JSON, but only
    /// for `/api/` requests.
    #[tokio::test]
    async fn test_client_errors() {
        let (parts, bytes) = request("/api/teapot").await.unwrap();
        assert_eq!(parts.status, StatusCode::IM_A_TEAPOT);
        assert_debug_snapshot!(parts.headers, @r###"
        {
            "content-type": "application/json",
            "content-length": "38",
        }
        "###);
        assert_debug_snapshot!(bytes, @r###"b"{\"errors\":[{\"detail\":\"I'm a teapot\"}]}""###);

        let (parts, bytes) = request("/teapot").await.unwrap();
        assert_eq!(parts.status, StatusCode::IM_A_TEAPOT);
        assert_debug_snapshot!(parts.headers, @r###"
        {
            "content-type": "text/plain; charset=utf-8",
            "content-length": "12",
        }
        "###);
        assert_debug_snapshot!(bytes, @r###"b"I'm a teapot""###);
    }

    /// Check that 5xx text responses **are** converted to JSON, but only
    /// for `/api/` requests.
    #[tokio::test]
    async fn test_server_errors() {
        let (parts, bytes) = request("/api/500").await.unwrap();
        assert_eq!(parts.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_debug_snapshot!(parts.headers, @r###"
        {
            "content-type": "application/json",
            "content-length": "47",
        }
        "###);
        assert_debug_snapshot!(bytes, @r###"b"{\"errors\":[{\"detail\":\"Internal Server Error\"}]}""###);

        let (parts, bytes) = request("/500").await.unwrap();
        assert_eq!(parts.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_debug_snapshot!(parts.headers, @r###"
        {
            "content-type": "text/plain; charset=utf-8",
            "content-length": "21",
        }
        "###);
        assert_debug_snapshot!(bytes, @r###"b"Internal Server Error""###);
    }
}
