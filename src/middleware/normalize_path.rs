//! Normalize request path if necessary

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use http::Uri;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OriginalPath(pub String);

pub async fn normalize_path(mut req: Request, next: Next) -> Response {
    normalize_path_inner(&mut req);
    next.run(req).await
}

fn normalize_path_inner(req: &mut Request) {
    let uri = req.uri();
    let path = uri.path();
    if path.contains("//") || path.contains("/.") {
        let original_path = OriginalPath(path.to_string());

        let path = Path::new(path)
            .components()
            .fold(
                PathBuf::with_capacity(path.len()),
                |mut result, p| match p {
                    Component::Normal(x) => {
                        if !x.is_empty() {
                            result.push(x)
                        };
                        result
                    }
                    Component::ParentDir => {
                        result.pop();
                        result
                    }
                    Component::RootDir => {
                        result.push(Component::RootDir);
                        result
                    }
                    _ => result,
                },
            )
            .to_string_lossy()
            .to_string(); // non-Unicode is replaced with U+FFFD REPLACEMENT CHARACTER

        let new_path_and_query = uri.path_and_query().map(|path_and_query| {
            match path_and_query.query() {
                Some(query) => format!("{path}?{query}"),
                None => path,
            }
            .parse()
            .unwrap()
        });

        let mut parts = uri.clone().into_parts();
        parts.path_and_query = new_path_and_query;

        if let Ok(new_uri) = Uri::from_parts(parts) {
            *req.uri_mut() = new_uri;
            req.extensions_mut().insert(original_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{OriginalPath, normalize_path_inner};
    use axum::body::Body;
    use axum::extract::Request;

    #[test]
    fn path_normalization() {
        let mut req = Request::get("/api/v1/.").body(Body::empty()).unwrap();
        normalize_path_inner(&mut req);
        assert_eq!(req.uri().path(), "/api/v1");
        assert_eq!(
            assert_some!(req.extensions().get::<OriginalPath>()).0,
            "/api/v1/."
        );

        let mut req = Request::get("/api/./v1").body(Body::empty()).unwrap();
        normalize_path_inner(&mut req);
        assert_eq!(req.uri().path(), "/api/v1");
        assert_eq!(
            assert_some!(req.extensions().get::<OriginalPath>()).0,
            "/api/./v1"
        );

        let mut req = Request::get("//api/v1/../v2").body(Body::empty()).unwrap();
        normalize_path_inner(&mut req);
        assert_eq!(req.uri().path(), "/api/v2");
        assert_eq!(
            assert_some!(req.extensions().get::<OriginalPath>()).0,
            "//api/v1/../v2"
        );
    }
}
