#![deny(warnings, clippy::all, rust_2018_idioms)]

extern crate base64;
extern crate chrono;
extern crate openssl;
extern crate reqwest;

use base64::encode;
use chrono::prelude::Utc;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::Signer;
use reqwest::header;

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

    pub fn put(
        &self,
        client: &reqwest::Client,
        path: &str,
        content: Vec<u8>,
        content_type: &str,
    ) -> reqwest::Result<reqwest::Response> {
        let path = if path.starts_with('/') {
            &path[1..]
        } else {
            path
        };
        let date = Utc::now().to_rfc2822();
        let auth = self.auth("PUT", &date, path, "", content_type);
        let url = self.url(path);

        client
            .put(&url)
            .header(header::AUTHORIZATION, auth)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::DATE, date)
            .body(content)
            .send()?
            .error_for_status()
    }

    pub fn delete(
        &self,
        client: &reqwest::Client,
        path: &str,
    ) -> reqwest::Result<reqwest::Response> {
        let path = if path.starts_with('/') {
            &path[1..]
        } else {
            path
        };
        let date = Utc::now().to_rfc2822();
        let auth = self.auth("DELETE", &date, path, "", "");
        let url = self.url(path);

        client
            .delete(&url)
            .header(header::DATE, date)
            .header(header::AUTHORIZATION, auth)
            .send()?
            .error_for_status()
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

    fn auth(&self, verb: &str, date: &str, path: &str, md5: &str, content_type: &str) -> String {
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
            let key = PKey::hmac(self.secret_key.as_bytes()).unwrap();
            let mut signer = Signer::new(MessageDigest::sha1(), &key).unwrap();
            signer.update(string.as_bytes()).unwrap();
            encode(&signer.sign_to_vec().unwrap()[..])
        };
        format!("AWS {}:{}", self.access_key, signature)
    }

    fn url(&self, path: &str) -> String {
        format!("{}://{}/{}", self.proto, self.host(), path)
    }
}
