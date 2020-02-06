#![warn(clippy::all, rust_2018_idioms)]

use base64::encode;
use chrono::prelude::Utc;
use openssl::error::ErrorStack;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::Signer;
use reqwest::{
    blocking::{Body, Client, Response},
    header,
};

mod error;
pub use error::Error;

#[derive(Clone, Debug)]
pub struct Bucket {
    name: String,
    region: Option<String>,
    access_key: String,
    secret_key: String,
    proto: String,
}

impl Bucket {
    pub fn new(
        name: String,
        region: Option<String>,
        access_key: String,
        secret_key: String,
        proto: &str,
    ) -> Bucket {
        Bucket {
            name,
            region,
            access_key,
            secret_key,
            proto: proto.to_string(),
        }
    }

    pub fn put<R: std::io::Read + Send + 'static>(
        &self,
        client: &Client,
        path: &str,
        content: R,
        content_length: u64,
        content_type: &str,
        extra_headers: header::HeaderMap,
    ) -> Result<Response, Error> {
        let path = if path.starts_with('/') {
            &path[1..]
        } else {
            path
        };
        let date = Utc::now().to_rfc2822();
        let auth = self.auth("PUT", &date, path, "", content_type)?;
        let url = self.url(path);

        client
            .put(&url)
            .header(header::AUTHORIZATION, auth)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::DATE, date)
            .header(header::USER_AGENT, "crates.io (https://crates.io)")
            .headers(extra_headers)
            .body(Body::sized(content, content_length))
            .send()?
            .error_for_status()
            .map_err(Into::into)
    }

    pub fn delete(&self, client: &Client, path: &str) -> Result<Response, Error> {
        let path = if path.starts_with('/') {
            &path[1..]
        } else {
            path
        };
        let date = Utc::now().to_rfc2822();
        let auth = self.auth("DELETE", &date, path, "", "")?;
        let url = self.url(path);

        client
            .delete(&url)
            .header(header::DATE, date)
            .header(header::AUTHORIZATION, auth)
            .send()?
            .error_for_status()
            .map_err(Into::into)
    }

    pub fn host(&self) -> String {
        format!(
            "{}.s3{}.amazonaws.com",
            self.name,
            match self.region {
                Some(ref r) if r != "" => format!("-{}", r),
                Some(_) => String::new(),
                None => String::new(),
            }
        )
    }

    fn auth(
        &self,
        verb: &str,
        date: &str,
        path: &str,
        md5: &str,
        content_type: &str,
    ) -> Result<String, ErrorStack> {
        let string = format!(
            "{verb}\n{md5}\n{ty}\n{date}\n{headers}{resource}",
            verb = verb,
            md5 = md5,
            ty = content_type,
            date = date,
            headers = "",
            resource = format!("/{}/{}", self.name, path)
        );
        let signature = {
            let key = PKey::hmac(self.secret_key.as_bytes())?;
            let mut signer = Signer::new(MessageDigest::sha1(), &key)?;
            signer.update(string.as_bytes())?;
            encode(&signer.sign_to_vec()?[..])
        };
        Ok(format!("AWS {}:{}", self.access_key, signature))
    }

    fn url(&self, path: &str) -> String {
        format!("{}://{}/{}", self.proto, self.host(), path)
    }
}
