#[derive(Clone, Debug)]
pub enum Uploader {
    /// For production usage, uploads and redirects to s3.
    /// For test usage with `TestApp::with_proxy()`, the recording proxy is used.
    S3 {
        bucket: Box<s3::Bucket>,
        index_bucket: Option<Box<s3::Bucket>>,
        cdn: Option<String>,
    },

    /// For development usage only: "uploads" crate files to `dist` and serves them
    /// from there as well to enable local publishing and download
    Local,
}
