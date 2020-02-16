#![allow(trivial_casts)]
#![cfg_attr(test, deny(warnings))]

extern crate conduit;
extern crate conduit_mime_types as mime;
extern crate time;
extern crate filetime;
#[cfg(test)] extern crate tempdir;

use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::path::{PathBuf, Path};
use std::fs::File;
use conduit::{Request, Response, Handler};
use filetime::FileTime;

pub struct Static {
    path: PathBuf,
    types: mime::Types
}

impl Static {
    pub fn new<P: AsRef<Path>>(path: P) -> Static {
        Static {
            path: path.as_ref().to_path_buf(),
            types: mime::Types::new()
                .ok().expect("Couldn't load mime-types")
        }
    }
}

impl Handler for Static {
    #[allow(deprecated)]
    fn call(&self, request: &mut dyn Request) -> Result<Response, Box<dyn Error+Send>> {
        let request_path = &request.path()[1..];
        if request_path.contains("..") { return Ok(not_found()) }

        let path = self.path.join(request_path);
        let mime = self.types.mime_for_path(&path);
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(..) => return Ok(not_found()),
        };
        let data = try!(file.metadata().map_err(|e| Box::new(e) as Box<dyn Error+Send>));
        if data.is_dir() {
            return Ok(not_found())
        }
        let mtime = FileTime::from_last_modification_time(&data);
        let ts = time::Timespec {
            sec: mtime.seconds_relative_to_1970() as i64,
            nsec: mtime.nanoseconds() as i32,
        };
        let tm = time::at(ts).to_utc();

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), vec![mime.to_string()]);
        headers.insert("Content-Length".to_string(),
                       vec![data.len().to_string()]);
        headers.insert("Last-Modified".to_string(),
                       vec![tm.strftime("%a, %d %b %Y %T GMT").unwrap()
                              .to_string()]);

        Ok(Response {
            status: (200, "OK"),
            headers: headers,
            body: Box::new(file),
        })
    }
}

fn not_found() -> Response {
    let mut headers = HashMap::new();
    headers.insert("Content-Length".to_string(), vec!["0".to_string()]);
    headers.insert("Content-Type".to_string(), vec!["text/plain".to_string()]);
    Response {
        status: (404, "Not Found"),
        headers: headers,
        body: Box::new(io::empty()),
    }
}

#[cfg(test)]
mod tests {
    extern crate conduit_test as test;

    use std::fs::{self, File};
    use std::io::prelude::*;
    use tempdir::TempDir;

    use conduit::{Handler, Method};
    use Static;

    #[test]
    fn test_static() {
        let td = TempDir::new("conduit-static").unwrap();
        let root = td.path();
        let handler = Static::new(root.clone());
        File::create(&root.join("Cargo.toml")).unwrap()
             .write_all(b"[package]").unwrap();
        let mut req = test::MockRequest::new(Method::Get, "/Cargo.toml");
        let mut res = handler.call(&mut req).ok().expect("No response");
        let mut body = Vec::new();
        res.body.write_body(&mut body).unwrap();
        assert_eq!(body, b"[package]");
        assert_eq!(res.headers.get("Content-Type"),
                   Some(&vec!("text/plain".to_string())));
        assert_eq!(res.headers.get("Content-Length"),
                   Some(&vec!["9".to_string()]));
    }

    #[test]
    fn test_mime_types() {
        let td = TempDir::new("conduit-static").unwrap();
        let root = td.path();
        fs::create_dir(&root.join("src")).unwrap();
        File::create(&root.join("src/fixture.css")).unwrap();

        let handler = Static::new(root.clone());
        let mut req = test::MockRequest::new(Method::Get, "/src/fixture.css");
        let res = handler.call(&mut req).ok().expect("No response");
        assert_eq!(res.headers.get("Content-Type"),
                   Some(&vec!("text/css".to_string())));
        assert_eq!(res.headers.get("Content-Length"),
                   Some(&vec!["0".to_string()]));
    }

    #[test]
    fn test_missing() {
        let td = TempDir::new("conduit-static").unwrap();
        let root = td.path();

        let handler = Static::new(root.clone());
        let mut req = test::MockRequest::new(Method::Get, "/nope");
        let res = handler.call(&mut req).ok().expect("No response");
        assert_eq!(res.status.0, 404);
    }

    #[test]
    fn test_dir() {
        let td = TempDir::new("conduit-static").unwrap();
        let root = td.path();

        fs::create_dir(&root.join("foo")).unwrap();

        let handler = Static::new(root.clone());
        let mut req = test::MockRequest::new(Method::Get, "/foo");
        let res = handler.call(&mut req).ok().expect("No response");
        assert_eq!(res.status.0, 404);
    }

    #[test]
    fn last_modified() {
        let td = TempDir::new("conduit-static").unwrap();
        let root = td.path();
        File::create(&root.join("test")).unwrap();
        let handler = Static::new(root.clone());
        let mut req = test::MockRequest::new(Method::Get, "/test");
        let res = handler.call(&mut req).ok().expect("No response");
        assert_eq!(res.status.0, 200);
        assert!(res.headers.get("Last-Modified").is_some());
    }
}
