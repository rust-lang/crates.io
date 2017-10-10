//! This module implements middleware for adding secuirty headers to
//! http responses in production.

use conduit::{Request, Response};
use conduit_middleware::Middleware;

use std::error::Error;
use std::collections::HashMap;

use Uploader;

#[derive(Clone, Debug)]
pub struct SecurityHeadersMiddleware {
    headers: HashMap<String, Vec<String>>,
}

impl SecurityHeadersMiddleware {
    pub fn new(uploader: &Uploader) -> Self {
        let mut headers = HashMap::new();

        headers.insert("X-Content-Type-Options".into(), vec!["nosniff".into()]);

        headers.insert("X-Frame-Options".into(), vec!["SAMEORIGIN".into()]);

        headers.insert("X-XSS-Protection".into(), vec!["1; mode=block".into()]);

        let s3_host = match *uploader {
            Uploader::S3 { ref bucket, .. } => bucket.host(),
            _ => unreachable!(
                "This middleware should only be used in the production environment, \
                 which should also require an S3 uploader, QED"
            ),
        };

        // It would be better if we didn't have to have 'unsafe-eval' in the `script-src`
        // policy, but google charts (used for the download graph on crate pages) uses `eval`
        // to load scripts. Remove 'unsafe-eval' if google fixes the issue:
        // https://github.com/google/google-visualization-issues/issues/1356
        // or if we switch to a different graph generation library.
        headers.insert(
            "Content-Security-Policy".into(),
            vec![
                format!(
                    "default-src 'self'; \
                     connect-src 'self' https://docs.rs https://{}; \
                     script-src 'self' 'unsafe-eval' \
                     https://www.google-analytics.com https://www.google.com; \
                     style-src 'self' https://www.google.com https://ajax.googleapis.com; \
                     img-src *; \
                     object-src 'none'",
                    s3_host
                ),
            ],
        );

        SecurityHeadersMiddleware { headers }
    }
}

impl Middleware for SecurityHeadersMiddleware {
    fn after(
        &self,
        _: &mut Request,
        mut res: Result<Response, Box<Error + Send>>,
    ) -> Result<Response, Box<Error + Send>> {
        if let Ok(ref mut response) = res {
            response.headers.extend(self.headers.clone());
        }
        res
    }
}
