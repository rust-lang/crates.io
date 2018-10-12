use conduit::Request;
use flate2::read::GzDecoder;
use openssl::hash::{Hasher, MessageDigest};
use semver;
use tar;

use util::LimitErrorReader;
use util::{human, internal, CargoResult, ChainError, Maximums};

use std::env;
use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::sync::Arc;

use futures::future::{result, FutureResult};
use futures::{Future, Poll};

use rusoto_core::request::HttpClient;
use rusoto_core::{ProvideAwsCredentials, Region};
use rusoto_credential::{AwsCredentials, CredentialsError};
use rusoto_s3::{DeleteObjectRequest, PutObjectRequest, S3Client, S3};

use app::App;

use middleware::app::RequestApp;
use models::Crate;

#[derive(Clone)]
pub enum Uploader {
    /// For production usage, uploads and redirects to s3.
    /// For test usage with a proxy.
    S3 {
        client: Arc<S3Client>,
        bucket: String,
        host: String,
        cdn: Option<String>,
        proxy: Option<String>,
    },

    /// For development usage only: "uploads" crate files to `dist` and serves them
    /// from there as well to enable local publishing and download
    Local,

    /// For one-off scripts where creating a Config is needed, but uploading is not.
    NoOp,
}

impl fmt::Debug for Uploader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Uploader::S3 { .. } => write!(f, "Uploader::S3"),
            Uploader::Local => write!(f, "Uploader::Local"),
            Uploader::NoOp => write!(f, "Uploader::NoOp"),
        }
    }
}

impl Uploader {
    /// Creates a new S3 uploader object.
    pub fn new_s3(
        bucket: String,
        region: Option<String>,
        access_key: String,
        secret_key: String,
        host: Option<String>,
        proto: String,
        cdn: Option<String>,
        proxy: Option<String>,
    ) -> Uploader {
        let host = host.unwrap_or_else(|| {
            format!(
                "{}://{}.s3{}.amazonaws.com",
                proto,
                bucket,
                match region {
                    Some(ref r) if r != "" => format!("-{}", r),
                    Some(_) => String::new(),
                    None => String::new(),
                }
            )
        });

        // Use the custom handler as we always provide an endpoint to connect to.
        let region = Region::Custom {
            name: region.unwrap_or_else(|| "us-east-1".to_string()),
            endpoint: host.clone(),
        };

        let dispatcher = HttpClient::new().expect("failed to create request dispatcher");
        let credentials = S3CredentialsProvider::new(access_key, secret_key);

        let s3client = S3Client::new_with(dispatcher, credentials, region);

        Uploader::S3 {
            client: Arc::new(s3client),
            bucket,
            host,
            cdn,
            proxy,
        }
    }

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
            Uploader::S3 {
                ref host,
                ref bucket,
                ref cdn,
                ..
            } => {
                let host = match *cdn {
                    Some(ref s) => s.clone(),
                    None => host.clone(),
                };

                let path = Uploader::crate_path(crate_name, version);
                Some(format!("https://{}/{}/{}", host, bucket, path))
            }
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
            Uploader::S3 {
                ref host,
                ref bucket,
                ref cdn,
                ..
            } => {
                let host = match *cdn {
                    Some(ref s) => s.clone(),
                    None => host.clone(),
                };
                let path = Uploader::readme_path(crate_name, version);
                Some(format!("https://{}/{}/{}", host, bucket, path))
            }
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
        path: &str,
        body: &[u8],
        content_type: &str,
        file_length: u64,
    ) -> CargoResult<(Option<String>, Vec<u8>)> {
        let hash = hash(body);
        match *self {
            Uploader::S3 {
                ref client,
                ref bucket,
                ..
            } => {
                let req = PutObjectRequest {
                    bucket: bucket.to_string(),
                    key: path.to_string(),
                    content_type: Some(content_type.to_string()),
                    content_length: Some(file_length as i64),
                    body: Some(body.to_vec().into()),
                    ..Default::default()
                };

                client.put_object(req).sync().chain_error(|| {
                    internal(&format_args!("failed to upload to S3: `{}`", path))
                })?;

                Ok((Some(String::from(path)), hash))
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
        req: &mut dyn Request,
        krate: &Crate,
        readme: Option<String>,
        file_length: u32,
        maximums: Maximums,
        vers: &semver::Version,
    ) -> CargoResult<(Vec<u8>, Bomb, Bomb)> {
        let app = Arc::clone(req.app());
        let (crate_path, checksum) = {
            let path = Uploader::crate_path(&krate.name, &vers.to_string());
            let mut body = Vec::new();
            LimitErrorReader::new(req.body(), maximums.max_upload_size).read_to_end(&mut body)?;
            verify_tarball(krate, vers, &body, maximums.max_unpack_size)?;
            self.upload(&path, &body, "application/x-tar", u64::from(file_length))?
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
            self.upload(&path, rendered.as_bytes(), "text/html", length as u64)?
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
    fn delete(&self, path: &str) -> CargoResult<()> {
        match *self {
            Uploader::S3 {
                ref client,
                ref bucket,
                ..
            } => {
                let req = DeleteObjectRequest {
                    bucket: bucket.to_string(),
                    key: path.to_string(),
                    ..Default::default()
                };

                client.delete_object(req).sync().chain_error(|| {
                    internal(&format_args!("failed to upload to S3: `{}`", path))
                })?;

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

#[derive(Clone, Debug)]
pub struct S3CredentialsProvider {
    access_key: String,
    secret_key: String,
}

impl S3CredentialsProvider {
    fn new(access_key: String, secret_key: String) -> S3CredentialsProvider {
        S3CredentialsProvider {
            access_key,
            secret_key,
        }
    }
}

/// Provides AWS credentials from an S3CredentialsProvider object as a Future.
#[derive(Debug)]
pub struct S3CredentialsProviderFuture {
    inner: FutureResult<AwsCredentials, CredentialsError>,
}

impl Future for S3CredentialsProviderFuture {
    type Item = AwsCredentials;
    type Error = CredentialsError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll()
    }
}

impl ProvideAwsCredentials for S3CredentialsProvider {
    type Future = S3CredentialsProviderFuture;

    fn credentials(&self) -> Self::Future {
        let access_key = self.access_key.clone();
        let secret_key = self.secret_key.clone();

        S3CredentialsProviderFuture {
            inner: result(Ok(AwsCredentials::new(access_key, secret_key, None, None))),
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
            if let Err(e) = self.app.config.uploader.delete(path) {
                println!("unable to delete {}, {:?}", path, e);
            }
        }
    }
}

fn verify_tarball(
    krate: &Crate,
    vers: &semver::Version,
    tarball: &[u8],
    max_unpack: u64,
) -> CargoResult<()> {
    // All our data is currently encoded with gzip
    let decoder = GzDecoder::new(tarball);

    // Don't let gzip decompression go into the weeeds, apply a fixed cap after
    // which point we say the decompressed source is "too large".
    let decoder = LimitErrorReader::new(decoder, max_unpack);

    // Use this I/O object now to take a peek inside
    let mut archive = tar::Archive::new(decoder);
    let prefix = format!("{}-{}", krate.name, vers);
    for entry in archive.entries()? {
        let entry = entry.chain_error(|| {
            human("uploaded tarball is malformed or too large when decompressed")
        })?;

        // Verify that all entries actually start with `$name-$vers/`.
        // Historically Cargo didn't verify this on extraction so you could
        // upload a tarball that contains both `foo-0.1.0/` source code as well
        // as `bar-0.1.0/` source code, and this could overwrite other crates in
        // the registry!
        if !entry.path()?.starts_with(&prefix) {
            return Err(human("invalid tarball uploaded"));
        }

        // Historical versions of the `tar` crate which Cargo uses internally
        // don't properly prevent hard links and symlinks from overwriting
        // arbitrary files on the filesystem. As a bit of a hammer we reject any
        // tarball with these sorts of links. Cargo doesn't currently ever
        // generate a tarball with these file types so this should work for now.
        let entry_type = entry.header().entry_type();
        if entry_type.is_hard_link() || entry_type.is_symlink() {
            return Err(human("invalid tarball uploaded"));
        }
    }
    Ok(())
}

fn hash(data: &[u8]) -> Vec<u8> {
    let mut hasher = Hasher::new(MessageDigest::sha256()).unwrap();
    hasher.update(data).unwrap();
    hasher.finish2().unwrap().to_vec()
}
