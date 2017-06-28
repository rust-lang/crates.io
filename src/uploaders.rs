use conduit::Request;
use krate::Crate;
use util::{CargoResult, internal, ChainError};
use util::{LimitErrorReader, HashingReader, read_le_u32};
use s3;
use semver;
use app::{App, RequestApp};
use std::sync::Arc;
use std::fs::{self, File};
use std::env;
use std::io;

#[derive(Clone, Debug)]
pub enum Uploader {
    /// For production usage, uploads and redirects to s3.
    /// For test usage with a proxy.
    S3 {
        bucket: s3::Bucket,
        proxy: Option<String>,
    },

    /// For development usage only: "uploads" crate files to `dist` and serves them
    /// from there as well to enable local publishing and download
    Local,

    /// For one-off scripts where creating a Config is needed, but uploading is not.
    NoOp,
}

impl Uploader {
    pub fn proxy(&self) -> Option<&str> {
        match *self {
            Uploader::S3 { ref proxy, .. } => proxy.as_ref().map(String::as_str),
            Uploader::Local | Uploader::NoOp => None,
        }
    }

    pub fn crate_location(&self, crate_name: &str, version: &str) -> Option<String> {
        match *self {
            Uploader::S3 { ref bucket, .. } => {
                Some(format!(
                    "https://{}/{}",
                    bucket.host(),
                    Uploader::crate_path(crate_name, version)
                ))
            }
            Uploader::Local => {
                Some(format!(
                    "/local_uploads/{}",
                    Uploader::crate_path(crate_name, version)
                ))
            }
            Uploader::NoOp => None,
        }
    }

    fn crate_path(name: &str, version: &str) -> String {
        // No slash in front so we can use join
        format!("crates/{}/{}-{}.crate", name, name, version)
    }

    pub fn upload(
        &self,
        req: &mut Request,
        krate: &Crate,
        max: u64,
        vers: &semver::Version,
    ) -> CargoResult<(Vec<u8>, Bomb)> {
        match *self {
            Uploader::S3 { ref bucket, .. } => {
                let mut handle = req.app().handle();
                let path = format!("/{}", Uploader::crate_path(&krate.name, &vers.to_string()));
                let (response, cksum) = {
                    let length = read_le_u32(req.body())?;
                    let body = LimitErrorReader::new(req.body(), max);
                    let mut body = HashingReader::new(body);
                    let mut response = Vec::new();
                    {
                        let mut s3req = bucket.put(
                            &mut handle,
                            &path,
                            &mut body,
                            "application/x-tar",
                            length as u64,
                        );
                        s3req
                            .write_function(|data| {
                                response.extend(data);
                                Ok(data.len())
                            })
                            .unwrap();
                        s3req.perform().chain_error(|| {
                            internal(&format_args!("failed to upload to S3: `{}`", path))
                        })?;
                    }
                    (response, body.finalize())
                };
                if handle.response_code().unwrap() != 200 {
                    let response = String::from_utf8_lossy(&response);
                    return Err(internal(&format_args!(
                        "failed to get a 200 response from S3: {}",
                        response
                    )));
                }

                Ok((
                    cksum,
                    Bomb {
                        app: req.app().clone(),
                        path: Some(path),
                    },
                ))
            }
            Uploader::Local => {
                let path = Uploader::crate_path(&krate.name, &vers.to_string());
                let crate_filename = env::current_dir()
                    .unwrap()
                    .join("dist")
                    .join("local_uploads")
                    .join(path);

                let crate_dir = crate_filename.parent().unwrap();
                fs::create_dir_all(crate_dir)?;

                let mut crate_file = File::create(&crate_filename)?;

                let cksum = {
                    read_le_u32(req.body())?;
                    let body = LimitErrorReader::new(req.body(), max);
                    let mut body = HashingReader::new(body);

                    io::copy(&mut body, &mut crate_file)?;
                    body.finalize()
                };

                Ok((
                    cksum,
                    Bomb {
                        app: req.app().clone(),
                        path: crate_filename.to_str().map(String::from),
                    },
                ))
            }
            Uploader::NoOp => {
                Ok((
                    vec![],
                    Bomb {
                        app: req.app().clone(),
                        path: None,
                    },
                ))
            }
        }
    }

    pub fn delete(&self, app: Arc<App>, path: &str) -> CargoResult<()> {
        match *self {
            Uploader::S3 { ref bucket, .. } => {
                let mut handle = app.handle();
                bucket.delete(&mut handle, path).perform()?;
                Ok(())
            }
            Uploader::Local => {
                fs::remove_file(path)?;
                Ok(())
            }
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
            if let Err(e) = self.app.config.uploader.delete(self.app.clone(), path) {
                println!("unable to delete {}, {:?}", path, e);
            }
        }
    }
}
