use conduit::Request;
use curl::easy::Easy;
use flate2::read::GzDecoder;
use krate::Crate;
use s3;
use semver;
use tar;
use util::{human, internal, CargoResult, ChainError};
use util::{hash, LimitErrorReader, read_le_u32};

use app::{App, RequestApp};
use std::sync::Arc;
use std::fs::{self, File};
use std::env;
use std::io::{Read, Write};

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

    /// Returns the URL of an uploaded crate's version archive.
    ///
    /// The function doesn't check for the existence of the file.
    /// It returns `None` if the current `Uploader` is `NoOp`.
    pub fn crate_location(&self, crate_name: &str, version: &str) -> Option<String> {
        match *self {
            Uploader::S3 { ref bucket, .. } => Some(format!(
                "https://{}/{}",
                bucket.host(),
                Uploader::crate_path(crate_name, version)
            )),
            Uploader::Local => Some(format!("/{}", Uploader::crate_path(crate_name, version))),
            Uploader::NoOp => None,
        }
    }

    /// Returns the URL of an uploaded crate's version readme.
    ///
    /// The function doesn't check for the existence of the file.
    /// It returns `None` if the current `Uploader` is `NoOp`.
    pub fn readme_location(&self, crate_name: &str, version: &str) -> Option<String> {
        match *self {
            Uploader::S3 { ref bucket, .. } => Some(format!(
                "https://{}/{}",
                bucket.host(),
                Uploader::readme_path(crate_name, version)
            )),
            Uploader::Local => Some(format!("/{}", Uploader::readme_path(crate_name, version))),
            Uploader::NoOp => None,
        }
    }

    /// Returns the interna path of an uploaded crate's version archive.
    fn crate_path(name: &str, version: &str) -> String {
        // No slash in front so we can use join
        format!("crates/{}/{}-{}.crate", name, name, version)
    }

    /// Returns the interna path of an uploaded crate's version readme.
    fn readme_path(name: &str, version: &str) -> String {
        format!("readmes/{}/{}-{}.html", name, name, version)
    }

    /// Uploads a file using the configured uploader (either `S3`, `Local` or `NoOp`).
    ///
    /// It returns a a tuple containing the path of the uploaded file
    /// and its checksum.
    pub fn upload(
        &self,
        mut handle: Easy,
        path: &str,
        body: &[u8],
        content_type: &str,
        content_length: u64,
    ) -> CargoResult<(Option<String>, Vec<u8>)> {
        let hash = hash(body);
        match *self {
            Uploader::S3 { ref bucket, .. } => {
                let (response, cksum) = {
                    let mut response = Vec::new();
                    {
                        let mut s3req =
                            bucket.put(&mut handle, path, body, content_type, content_length);
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
                    (response, hash)
                };
                if handle.response_code().unwrap() != 200 {
                    let response = String::from_utf8_lossy(&response);
                    return Err(internal(&format_args!(
                        "failed to get a 200 response from S3: {}",
                        response
                    )));
                }
                Ok((Some(String::from(path)), cksum))
            }
            Uploader::Local => {
                let filename = env::current_dir().unwrap().join("local_uploads").join(path);
                let dir = filename.parent().unwrap();
                fs::create_dir_all(dir)?;
                let mut file = File::create(&filename)?;
                file.write_all(body)?;
                Ok((filename.to_str().map(String::from), hash))
            }
            Uploader::NoOp => Ok((None, vec![])),
        }
    }

    /// Uploads a crate and its readme. Returns the checksum of the uploaded crate
    /// file, and bombs for the uploaded crate and the uploaded readme.
    pub fn upload_crate(
        &self,
        req: &mut Request,
        krate: &Crate,
        readme: Option<String>,
        max: u64,
        vers: &semver::Version,
    ) -> CargoResult<(Vec<u8>, Bomb, Bomb)> {
        let app = Arc::clone(req.app());
        let (crate_path, checksum) = {
            let path = Uploader::crate_path(&krate.name, &vers.to_string());
            let length = read_le_u32(req.body())?;
            let mut body = Vec::new();
            LimitErrorReader::new(req.body(), max).read_to_end(&mut body)?;
            verify_tarball(krate, vers, &body)?;
            self.upload(
                app.handle(),
                &path,
                &body,
                "application/x-tar",
                u64::from(length),
            )?
        };
        // We create the bomb for the crate file before uploading the readme so that if the
        // readme upload fails, the uploaded crate file is automatically deleted.
        let crate_bomb = Bomb {
            app: Arc::clone(&app),
            path: crate_path,
        };
        let (readme_path, _) = if let Some(rendered) = readme {
            let path = Uploader::readme_path(&krate.name, &vers.to_string());
            let length = rendered.len();
            self.upload(
                app.handle(),
                &path,
                rendered.as_bytes(),
                "text/html",
                length as u64,
            )?
        } else {
            (None, vec![])
        };
        Ok((
            checksum,
            crate_bomb,
            Bomb {
                app: Arc::clone(&app),
                path: readme_path,
            },
        ))
    }

    /// Deletes an uploaded file.
    fn delete(&self, app: Arc<App>, path: &str) -> CargoResult<()> {
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

// Can't derive Debug because of App.
#[allow(missing_debug_implementations)]
pub struct Bomb {
    app: Arc<App>,
    pub path: Option<String>,
}

impl Drop for Bomb {
    fn drop(&mut self) {
        if let Some(ref path) = self.path {
            if let Err(e) = self.app.config.uploader.delete(Arc::clone(&self.app), path) {
                println!("unable to delete {}, {:?}", path, e);
            }
        }
    }
}

fn verify_tarball(krate: &Crate, vers: &semver::Version, tarball: &[u8]) -> CargoResult<()> {
    let decoder = GzDecoder::new(tarball)?;
    let mut archive = tar::Archive::new(decoder);
    let prefix = format!("{}-{}", krate.name, vers);
    for entry in archive.entries()? {
        let entry = entry?;

        // Verify that all entries actually start with `$name-$vers/`.
        // Historically Cargo didn't verify this on extraction so you could
        // upload a tarball that contains both `foo-0.1.0/` source code as well
        // as `bar-0.1.0/` source code, and this could overwrite other crates in
        // the registry!
        if !entry.path()?.starts_with(&prefix) {
            return Err(human("invalid tarball uploaded"));
        }
    }
    Ok(())
}
