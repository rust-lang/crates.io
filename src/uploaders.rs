use conduit::Request;
use krate::Crate;
use util::{CargoResult, internal, ChainError};
use util::{LimitErrorReader, HashingReader, read_le_u32};
use s3;
use semver;
use app::{App, RequestApp};
use std::sync::Arc;

#[derive(Clone)]
pub enum Uploader {
    /// For production usage, uploads and redirects to s3.
    /// For test usage with a proxy.
    S3 { bucket: s3::Bucket, proxy: Option<String> },

    /// For one-off scripts where creating a Config is needed, but uploading is not.
    NoOp,
    // next: LocalUploader {},
}

impl Uploader {
    pub fn proxy(&self) -> Option<&str> {
        match *self {
            Uploader::S3 { ref proxy, .. } => proxy.as_ref().map(String::as_str),
            Uploader::NoOp => None,
        }
    }

    pub fn crate_location(&self, crate_name: &str, version: &str) -> String {
        match *self {
            Uploader::S3 { ref bucket, .. } => {
                format!("https://{}/crates/{}/{}-{}.crate",
                        bucket.host(),
                        crate_name, crate_name, version)
            },
            Uploader::NoOp => panic!("no-op uploader does not have crate files"),
        }
    }

    pub fn upload(&self, req: &mut Request, krate: &Crate, max: u64, vers: &semver::Version) -> CargoResult<(Vec<u8>, Bomb)> {
        match *self {
            Uploader::S3 { ref bucket, .. } => {
                let mut handle = req.app().handle();
                let path = krate.s3_path(&vers.to_string());
                let (response, cksum) = {
                    let length = read_le_u32(req.body())?;
                    let body = LimitErrorReader::new(req.body(), max);
                    let mut body = HashingReader::new(body);
                    let mut response = Vec::new();
                    {
                        let mut s3req = bucket.put(&mut handle, &path, &mut body,
                                                       "application/x-tar",
                                                       length as u64);
                        s3req.write_function(|data| {
                            response.extend(data);
                            Ok(data.len())
                        }).unwrap();
                        s3req.perform().chain_error(|| {
                            internal(format!("failed to upload to S3: `{}`", path))
                        })?;
                    }
                    (response, body.finalize())
                };
                if handle.response_code().unwrap() != 200 {
                    let response = String::from_utf8_lossy(&response);
                    return Err(internal(format!("failed to get a 200 response from S3: {}",
                                                response)))
                }

                Ok((cksum, Bomb {
                    app: req.app().clone(),
                    path: Some(path),
                }))
            },
            Uploader::NoOp => Ok((vec![], Bomb { app: req.app().clone(), path: None })),
        }
    }

    pub fn delete(&self, app: Arc<App>, path: &str) -> CargoResult<()> {
        match *self {
            Uploader::S3 { ref bucket, .. } => {
                let mut handle = app.handle();
                bucket.delete(&mut handle, path).perform()?;
                Ok(())
            },
            Uploader::NoOp => Ok(()),
        }
    }
}

pub struct Bomb {
    app: Arc<App>,
    pub path: Option<String>,
}

impl Drop for Bomb {
    fn drop(&mut self) {
        if let Some(ref path) = self.path {
            drop(self.app.config.uploader.delete(self.app.clone(), &path));
        }
    }
}
