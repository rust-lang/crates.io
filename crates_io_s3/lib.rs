#![warn(clippy::all, rust_2018_idioms)]

use base64::{engine::general_purpose, Engine};
use chrono::prelude::Utc;
use hmac::{Hmac, Mac};
use reqwest::{
    blocking::{Body, Client, Response},
    header,
};
use sha1::Sha1;
use std::time::Duration;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),
}

#[derive(Clone, Debug)]
pub struct Bucket {
    name: String,
    region: Region,
    access_key: String,
    secret_key: String,
    proto: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Region {
    Host(String),
    Region(String),
    Default,
}

impl Region {
    fn request_url(&self, proto: &str, bucket: &str, path: &str) -> Result<Url, Error> {
        Ok(Url::parse(&match self {
            Region::Host(host) => format!("{proto}://{host}/{bucket}/{path}"),
            Region::Region(region) => {
                format!("{proto}://{bucket}.s3-{region}.amazonaws.com/{path}")
            }
            Region::Default => format!("{proto}://{bucket}.s3.amazonaws.com/{path}"),
        })?)
    }
}

impl Bucket {
    pub fn new(
        name: String,
        region: Region,
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

    pub fn put<R: Into<Body>>(
        &self,
        client: &Client,
        path: &str,
        content: R,
        content_type: &str,
        extra_headers: header::HeaderMap,
    ) -> Result<Response, Error> {
        let path = path.strip_prefix('/').unwrap_or(path);
        let date = Utc::now().to_rfc2822();
        let auth = self.auth("PUT", &date, path, "", content_type);
        let url = self.url(path)?;

        client
            .put(url)
            .header(header::AUTHORIZATION, auth)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::DATE, date)
            .header(header::USER_AGENT, "crates.io (https://crates.io)")
            .headers(extra_headers)
            .body(content.into())
            .timeout(Duration::from_secs(60))
            .send()?
            .error_for_status()
            .map_err(Into::into)
    }

    pub fn delete(&self, client: &Client, path: &str) -> Result<Response, Error> {
        let path = path.strip_prefix('/').unwrap_or(path);
        let date = Utc::now().to_rfc2822();
        let auth = self.auth("DELETE", &date, path, "", "");
        let url = self.url(path)?;

        client
            .delete(url)
            .header(header::DATE, date)
            .header(header::AUTHORIZATION, auth)
            .send()?
            .error_for_status()
            .map_err(Into::into)
    }

    fn auth(&self, verb: &str, date: &str, path: &str, md5: &str, content_type: &str) -> String {
        let string = format!(
            "{verb}\n{md5}\n{ty}\n{date}\n{headers}/{name}/{path}",
            ty = content_type,
            headers = "",
            name = self.name,
        );
        let signature = {
            let key = self.secret_key.as_bytes();
            let mut h = Hmac::<Sha1>::new_from_slice(key).expect("HMAC can take key of any size");
            h.update(string.as_bytes());
            let res = h.finalize().into_bytes();
            general_purpose::STANDARD.encode(res)
        };
        format!("AWS {}:{}", self.access_key, signature)
    }

    pub fn url(&self, path: &str) -> Result<String, Error> {
        self.region
            .request_url(&self.proto, &self.name, path)
            .map(|url| url.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_url() -> Result<(), Error> {
        for (bucket, path, expected) in [
            (
                bucket("buckey", region("us-west-2"), "https"),
                "foo/bar",
                "https://buckey.s3-us-west-2.amazonaws.com/foo/bar",
            ),
            (
                bucket("buck-rogers", host("127.0.0.1:19000"), "http"),
                "foo/bar",
                "http://127.0.0.1:19000/buck-rogers/foo/bar",
            ),
            (
                bucket("buckminster-fuller", Region::Default, "gopher"),
                "",
                "gopher://buckminster-fuller.s3.amazonaws.com/",
            ),
        ] {
            assert_eq!(&bucket.url(path)?, expected);
        }

        Ok(())
    }

    fn bucket(name: &str, region: Region, proto: &str) -> Bucket {
        Bucket::new(name.into(), region, "".into(), "".into(), proto)
    }

    fn region(name: &str) -> Region {
        Region::Region(name.into())
    }

    fn host(host: &str) -> Region {
        Region::Host(host.into())
    }
}
