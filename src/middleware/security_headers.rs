//! Middleware that adds secuirty headers to http responses in production.

use super::prelude::*;
use crate::Uploader;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct SecurityHeaders {
    headers: HashMap<String, Vec<String>>,
}

impl SecurityHeaders {
    pub fn new(uploader: &Uploader) -> Self {
        let mut headers = HashMap::new();

        headers.insert("X-Content-Type-Options".into(), vec!["nosniff".into()]);

        headers.insert("X-Frame-Options".into(), vec!["SAMEORIGIN".into()]);

        headers.insert("X-XSS-Protection".into(), vec!["1; mode=block".into()]);

        let s3_host = match *uploader {
            Uploader::S3 {
                ref bucket,
                ref cdn,
                ..
            } => match *cdn {
                Some(ref s) => s.clone(),
                None => bucket.host(),
            },
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
            vec![format!(
                "default-src 'self'; \
                 connect-src 'self' https://docs.rs https://{}; \
                 script-src 'self' 'unsafe-eval' https://www.google.com; \
                 style-src 'self' https://www.google.com https://ajax.googleapis.com; \
                 img-src *; \
                 object-src 'none'",
                s3_host
            )],
        );

        SecurityHeaders { headers }
    }
}

impl Middleware for SecurityHeaders {
    fn after(
        &self,
        _: &mut dyn Request,
        mut res: Result<Response, Box<dyn Error + Send>>,
    ) -> Result<Response, Box<dyn Error + Send>> {
        if let Ok(ref mut response) = res {
            response.headers.extend(self.headers.clone());
        }
        res
    }
}
