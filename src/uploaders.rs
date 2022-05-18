use anyhow::Result;
use reqwest::{blocking::Client, header};

use crate::util::errors::{internal, AppResult};

use std::env;
use std::fs::{self, File};
use std::io::{Cursor, SeekFrom};

use crate::models::Crate;

const CACHE_CONTROL_IMMUTABLE: &str = "public,max-age=31536000,immutable";
const CACHE_CONTROL_README: &str = "public,max-age=604800";
const CACHE_CONTROL_INDEX: &str = "public,max-age=600";

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
        match *self {
            Uploader::S3 {
                ref bucket,
                ref cdn,
                ..
            } => {
                let host = match *cdn {
                    Some(ref s) => s.clone(),
                    None => bucket.host(),
                };
                let path = Uploader::crate_path(crate_name, version);
                format!("https://{host}/{path}")
            }
            Uploader::Local => format!("/{}", Uploader::crate_path(crate_name, version)),
        }
    }

    /// Returns the URL of an uploaded crate's version readme.
    ///
    /// The function doesn't check for the existence of the file.
    pub fn readme_location(&self, crate_name: &str, version: &str) -> String {
        match *self {
            Uploader::S3 {
                ref bucket,
                ref cdn,
                ..
            } => {
                let host = match *cdn {
                    Some(ref s) => s.clone(),
                    None => bucket.host(),
                };
                let path = Uploader::readme_path(crate_name, version);
                format!("https://{host}/{path}")
            }
            Uploader::Local => format!("/{}", Uploader::readme_path(crate_name, version)),
        }
    }

    /// Returns the internal path of an uploaded crate's version archive.
    fn crate_path(name: &str, version: &str) -> String {
        format!("crates/{name}/{name}-{version}.crate")
    }

    /// Returns the internal path of an uploaded crate's version readme.
    fn readme_path(name: &str, version: &str) -> String {
        format!("readmes/{name}/{name}-{version}.html")
    }

    /// Returns the internal path of an uploaded crate's index file.
    fn index_path(name: &str) -> String {
        let path = cargo_registry_index::Repository::relative_index_file_for_url(name);
        format!("index/{}", path)
    }

    /// Uploads a file using the configured uploader (either `S3`, `Local`).
    ///
    /// It returns the path of the uploaded file.
    ///
    /// # Panics
    ///
    /// This function can panic on an `Self::Local` during development.
    /// Production and tests use `Self::S3` which should not panic.
    pub fn upload<R: std::io::Read + std::io::Seek + Send + 'static>(
        &self,
        client: &Client,
        path: &str,
        mut content: R,
        content_type: &str,
        extra_headers: header::HeaderMap,
        upload_bucket: UploadBucket,
    ) -> Result<Option<String>> {
        match *self {
            Uploader::S3 {
                ref bucket,
                ref index_bucket,
                ..
            } => {
                let bucket = match upload_bucket {
                    UploadBucket::Default => Some(bucket),
                    UploadBucket::Index => index_bucket.as_ref(),
                };

                if let Some(bucket) = bucket {
                    let content_length = content.seek(SeekFrom::End(0))?;
                    content.seek(SeekFrom::Start(0))?;
                    bucket.put(
                        client,
                        path,
                        content,
                        content_length,
                        content_type,
                        extra_headers,
                    )?;
                }

                Ok(Some(String::from(path)))
            }
            Uploader::Local => {
                let filename = env::current_dir().unwrap().join("local_uploads").join(path);
                let dir = filename.parent().unwrap();
                fs::create_dir_all(dir)?;
                let mut file = File::create(&filename)?;
                std::io::copy(&mut content, &mut file)?;
                Ok(filename.to_str().map(String::from))
            }
        }
    }

    /// Deletes a file using the configured uploader (either `S3`, `Local`).
    pub fn delete(&self, client: &Client, path: &str, upload_bucket: UploadBucket) -> Result<()> {
        match *self {
            Uploader::S3 {
                ref bucket,
                ref index_bucket,
                ..
            } => {
                let bucket = match upload_bucket {
                    UploadBucket::Default => Some(bucket),
                    UploadBucket::Index => index_bucket.as_ref(),
                };

                if let Some(bucket) = bucket {
                    bucket.delete(client, path)?;
                }
            }
            Uploader::Local => {
                let filename = env::current_dir().unwrap().join("local_uploads").join(path);
                std::fs::remove_file(&filename)?;
            }
        }
        Ok(())
    }

    /// Uploads a crate and returns the checksum of the uploaded crate file.
    pub fn upload_crate(
        &self,
        http_client: &Client,
        body: Vec<u8>,
        krate: &Crate,
        vers: &semver::Version,
    ) -> AppResult<()> {
        let path = Uploader::crate_path(&krate.name, &vers.to_string());
        let content = Cursor::new(body);
        let mut extra_headers = header::HeaderMap::new();
        extra_headers.insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static(CACHE_CONTROL_IMMUTABLE),
        );
        self.upload(
            http_client,
            &path,
            content,
            "application/gzip",
            extra_headers,
            UploadBucket::Default,
        )
        .map_err(|e| internal(&format_args!("failed to upload crate: {}", e)))?;
        Ok(())
    }

    pub(crate) fn upload_readme(
        &self,
        http_client: &Client,
        crate_name: &str,
        vers: &str,
        readme: String,
    ) -> Result<()> {
        let path = Uploader::readme_path(crate_name, vers);
        let content = Cursor::new(readme);
        let mut extra_headers = header::HeaderMap::new();
        extra_headers.insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static(CACHE_CONTROL_README),
        );
        self.upload(
            http_client,
            &path,
            content,
            "text/html",
            extra_headers,
            UploadBucket::Default,
        )?;
        Ok(())
    }

    pub(crate) fn upload_index(
        &self,
        http_client: &Client,
        crate_name: &str,
        index: String,
    ) -> Result<()> {
        let path = Uploader::index_path(crate_name);
        let content = Cursor::new(index);
        let mut extra_headers = header::HeaderMap::new();
        extra_headers.insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static(CACHE_CONTROL_INDEX),
        );
        self.upload(
            http_client,
            &path,
            content,
            "text/plain",
            extra_headers,
            UploadBucket::Index,
        )?;
        Ok(())
    }

    pub(crate) fn delete_index(&self, http_client: &Client, crate_name: &str) -> Result<()> {
        let path = Uploader::index_path(crate_name);
        self.delete(http_client, &path, UploadBucket::Index)?;
        Ok(())
    }
}
