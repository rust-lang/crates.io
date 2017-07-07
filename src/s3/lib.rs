#![deny(warnings)]

extern crate time;
extern crate curl;
extern crate rustc_serialize;
extern crate openssl;

use std::io::prelude::*;

use curl::easy::{Easy, Transfer, List, ReadError};
use openssl::hash::MessageDigest;
use openssl::sign::Signer;
use openssl::pkey::PKey;
use rustc_serialize::base64::{ToBase64, STANDARD};

#[derive(Clone, Debug)]
pub struct Bucket {
    name: String,
    region: Option<String>,
    access_key: String,
    secret_key: String,
    proto: String,
}

impl Bucket {
    pub fn new(name: String,
               region: Option<String>,
               access_key: String,
               secret_key: String,
               proto: &str) -> Bucket {
        Bucket {
            name: name,
            region: region,
            access_key: access_key,
            secret_key: secret_key,
            proto: proto.to_string(),
        }
    }

    pub fn put<'a, 'b>(&self,
                       easy: &'a mut Easy,
                       path: &str,
                       content: &'b mut Read,
                       content_type: &str,
                       content_length: u64)
                       -> Transfer<'a, 'b> {
        let path = if path.starts_with("/") {&path[1..]} else {path};
        let host = self.host();
        let date = time::now().rfc822z().to_string();
        let auth = self.auth("PUT", &date, path, "", content_type);
        let url = format!("{}://{}/{}", self.proto, host, path);

        let mut headers = List::new();
        headers.append(&format!("Host: {}", host)).unwrap();
        headers.append(&format!("Date: {}", date)).unwrap();
        headers.append(&format!("Authorization: {}", auth)).unwrap();
        headers.append(&format!("Content-Type: {}", content_type)).unwrap();

        // Disable the `Expect: 100-continue` header for now, this cause
        // problems with the test harness currently and the purpose is
        // not yet clear. Would probably be good to reenable at some point.
        headers.append("Expect:").unwrap();

        easy.url(&url).unwrap();
        easy.put(true).unwrap();
        easy.http_headers(headers).unwrap();
        easy.upload(true).unwrap();
        easy.in_filesize(content_length).unwrap();

        let mut transfer = easy.transfer();
        transfer.read_function(move |data| {
            content.read(data).map_err(|_| ReadError::Abort)
        }).unwrap();

        return transfer
    }

    pub fn delete<'a, 'b>(&self,
                          easy: &'a mut Easy,
                          path: &str)
                          -> Transfer<'a, 'b> {
        let path = if path.starts_with("/") {&path[1..]} else {path};
        let host = self.host();
        let date = time::now().rfc822z().to_string();
        let auth = self.auth("DELETE", &date, path, "", "");
        let url = format!("{}://{}/{}", self.proto, host, path);

        let mut headers = List::new();
        headers.append(&format!("Host: {}", host)).unwrap();
        headers.append(&format!("Date: {}", date)).unwrap();
        headers.append(&format!("Authorization: {}", auth)).unwrap();

        easy.custom_request("DELETE").unwrap();
        easy.url(&url).unwrap();
        easy.http_headers(headers).unwrap();

        return easy.transfer()
    }

    pub fn host(&self) -> String {
        format!("{}.s3{}.amazonaws.com", self.name,
                match self.region {
                    Some(ref r) if r != "" => format!("-{}", r),
                    Some(_) => String::new(),
                    None => String::new(),
                })
    }

    fn auth(&self, verb: &str, date: &str, path: &str,
            md5: &str, content_type: &str) -> String {
        let string = format!("{verb}\n{md5}\n{ty}\n{date}\n{headers}{resource}",
                             verb = verb,
                             md5 = md5,
                             ty = content_type,
                             date = date,
                             headers = "",
                             resource = format!("/{}/{}", self.name, path));
        let signature = {
            let key = PKey::hmac(self.secret_key.as_bytes()).unwrap();
            let mut signer = Signer::new(MessageDigest::sha1(), &key).unwrap();
            signer.update(string.as_bytes()).unwrap();
            signer.finish().unwrap().to_base64(STANDARD)
        };
        format!("AWS {}:{}", self.access_key, signature)
    }
}
