//! Normalize request path if necessary

use super::prelude::*;

use std::path::{Component, Path, PathBuf};

pub struct NormalizePath;

impl Middleware for NormalizePath {
    fn before(&self, req: &mut dyn RequestExt) -> BeforeResult {
        let path = req.path();
        if !(path.contains("//") || path.contains("/.")) {
            // Avoid allocations if rewriting is unnecessary
            return Ok(());
        }

        let path = Path::new(path)
            .components()
            .fold(
                PathBuf::with_capacity(path.len()),
                |mut result, p| match p {
                    Component::Normal(x) => {
                        if x != "" {
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

        super::log_request::add_custom_metadata(req, "normalized_path", path.clone());
        *req.path_mut() = path;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::NormalizePath;

    use conduit::RequestExt;
    use conduit_middleware::Middleware;
    use conduit_test::MockRequest;

    #[test]
    fn path_normalization() {
        let mut req = MockRequest::new(::conduit::Method::GET, "/api/v1/.");
        let _ = NormalizePath.before(&mut req);
        assert_eq!(req.path(), "/api/v1");

        let mut req = MockRequest::new(::conduit::Method::GET, "/api/./v1");
        let _ = NormalizePath.before(&mut req);
        assert_eq!(req.path(), "/api/v1");

        let mut req = MockRequest::new(::conduit::Method::GET, "//api/v1/../v2");
        let _ = NormalizePath.before(&mut req);
        assert_eq!(req.path(), "/api/v2");
    }
}
