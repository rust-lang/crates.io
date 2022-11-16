use std::time::SystemTime;

use anyhow::Context;
use aws_sigv4::{
    http_request::{self, SignableRequest, SigningSettings},
    SigningParams,
};
use reqwest::blocking::Client;
use retry::delay::{jitter, Exponential};
use retry::OperationResult;

#[derive(Clone)]
pub struct CloudFront {
    distribution_id: String,
    access_key: String,
    secret_key: String,
}

impl CloudFront {
    pub fn from_environment() -> Option<Self> {
        let distribution_id = dotenv::var("CLOUDFRONT_DISTRIBUTION").ok()?;
        let access_key = dotenv::var("AWS_ACCESS_KEY").expect("missing AWS_ACCESS_KEY");
        let secret_key = dotenv::var("AWS_SECRET_KEY").expect("missing AWS_SECRET_KEY");
        Some(Self {
            distribution_id,
            access_key,
            secret_key,
        })
    }

    /// Invalidate a file on CloudFront
    ///
    /// `path` is the path to the file to invalidate, such as `config.json`, or `re/ge/regex`
    pub fn invalidate(&self, client: &Client, path: &str) -> anyhow::Result<()> {
        let path = path.trim_start_matches('/');
        let url = format!(
            "https://cloudfront.amazonaws.com/2020-05-31/distribution/{}/invalidation",
            self.distribution_id
        );

        let backoff = Exponential::from_millis(1000).map(jitter).take(2);
        retry::retry_with_index(backoff, |attempt| {
            if attempt > 1 {
                debug!(?attempt, "Retrying CloudFront invalidation…")
            }

            let now = chrono::offset::Utc::now().timestamp_micros();
            let body = format!(
                r#"
<?xml version="1.0" encoding="UTF-8"?>
<InvalidationBatch xmlns="http://cloudfront.amazonaws.com/doc/2020-05-31/">
    <CallerReference>{now}</CallerReference>
    <Paths>
        <Items>
            <Path>/{path}</Path>
        </Items>
        <Quantity>1</Quantity>
    </Paths>
</InvalidationBatch>
"#
            );

            let request = match http::Request::post(&url)
                .body(&body)
                .context("Failed to construct HTTP request")
            {
                Ok(request) => request,
                Err(error) => return OperationResult::Err(error),
            };

            let request = SignableRequest::from(&request);
            let params = SigningParams::builder()
                .access_key(&self.access_key)
                .secret_key(&self.secret_key)
                .region("us-east-1") // cloudfront is a regionless service, use the default region for signing.
                .service_name("cloudfront")
                .settings(SigningSettings::default())
                .time(SystemTime::now())
                .build()
                .unwrap(); // all required fields are set

            let (mut signature_headers, _) =
                http_request::sign(request, &params).unwrap().into_parts();

            let response = match client
                .post(&url)
                .headers(signature_headers.take_headers().unwrap_or_default())
                .body(body)
                .send()
                .context("Failed to send invalidation request")
            {
                Ok(response) => response,
                Err(error) => return OperationResult::Retry(error),
            };

            let status = response.status();

            let result = match response.error_for_status_ref() {
                Ok(_) => Ok(()),
                Err(error) => {
                    let headers = format!("{:?}", response.headers());
                    let body = response
                        .text()
                        .unwrap_or_else(|_| "Failed to decode response payload".to_string());

                    Err(error).context(format!("{headers}; body: {body}"))
                }
            };

            match result {
                Ok(_) => OperationResult::Ok(()),
                Err(error) if status.is_server_error() => OperationResult::Retry(error),
                Err(error) => OperationResult::Err(error),
            }
        })
        .map_err(|error| error.error)
    }
}
