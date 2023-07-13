use std::env;
use std::path::PathBuf;

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

pub enum UploadBucket {
    Default,
    Index,
}

impl Uploader {
    /// Returns the URL of an uploaded crate's version archive.
    ///
    /// The function doesn't check for the existence of the file.
    pub fn crate_location(&self, crate_name: &str, version: &str) -> String {
        let version = version.replace('+', "%2B");

        match *self {
            Uploader::S3 {
                ref bucket,
                ref cdn,
                ..
            } => {
                let path = Uploader::crate_path(crate_name, &version);
                match *cdn {
                    Some(ref host) => format!("https://{host}/{path}"),
                    None => bucket.url(&path).unwrap(),
                }
            }
            Uploader::Local => format!("/{}", Uploader::crate_path(crate_name, &version)),
        }
    }

    /// Returns the URL of an uploaded crate's version readme.
    ///
    /// The function doesn't check for the existence of the file.
    pub fn readme_location(&self, crate_name: &str, version: &str) -> String {
        let version = version.replace('+', "%2B");

        match *self {
            Uploader::S3 {
                ref bucket,
                ref cdn,
                ..
            } => {
                let path = Uploader::readme_path(crate_name, &version);
                match *cdn {
                    Some(ref host) => format!("https://{host}/{path}"),
                    None => bucket.url(&path).unwrap(),
                }
            }
            Uploader::Local => format!("/{}", Uploader::readme_path(crate_name, &version)),
        }
    }

    /// Returns the internal path of an uploaded crate's version archive.
    pub fn crate_path(name: &str, version: &str) -> String {
        format!("crates/{name}/{name}-{version}.crate")
    }

    /// Returns the internal path of an uploaded crate's version readme.
    pub fn readme_path(name: &str, version: &str) -> String {
        format!("readmes/{name}/{name}-{version}.html")
    }

    /// Returns the absolute path to the locally uploaded file.
    fn local_uploads_path(path: &str, upload_bucket: UploadBucket) -> PathBuf {
        let path = match upload_bucket {
            UploadBucket::Index => PathBuf::from("index").join(path),
            UploadBucket::Default => PathBuf::from(path),
        };
        env::current_dir().unwrap().join("local_uploads").join(path)
    }
}
