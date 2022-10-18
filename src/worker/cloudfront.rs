use std::time::SystemTime;

use aws_sigv4::{
    http_request::{self, SignableRequest, SigningSettings},
    SigningParams,
};
use reqwest::blocking::Client;

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

        let request = http::Request::post(&url).body(&body)?;
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
        let (mut signature_headers, _) = http_request::sign(request, &params).unwrap().into_parts();
        client
            .post(url)
            .headers(signature_headers.take_headers().unwrap_or_default())
            .body(body)
            .send()?
            .error_for_status()?;
        Ok(())
    }
}
