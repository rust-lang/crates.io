#![allow(trivial_casts)]
#![warn(rust_2018_idioms)]
#![cfg_attr(test, deny(warnings))]

extern crate conduit;
extern crate conduit_mime_types as mime;
extern crate filetime;
#[cfg(test)]
extern crate tempdir;
extern crate time;

use conduit::{box_error, header, Body, Handler, HandlerResult, RequestExt, Response, StatusCode};
use filetime::FileTime;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

pub struct Static {
    path: PathBuf,
    types: mime::Types,
}

impl Static {
    pub fn new<P: AsRef<Path>>(path: P) -> Static {
        Static {
            path: path.as_ref().to_path_buf(),
            types: mime::Types::new().expect("Couldn't load mime-types"),
        }
    }
}

impl Handler for Static {
    #[allow(deprecated)]
    fn call(&self, request: &mut dyn RequestExt) -> HandlerResult {
        let request_path = &request.path()[1..];
        if request_path.contains("..") {
            return Ok(not_found());
        }

        let path = self.path.join(request_path);
        let mime = self.types.mime_for_path(&path);
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(..) => return Ok(not_found()),
        };
        let data = file.metadata().map_err(box_error)?;
        if data.is_dir() {
            return Ok(not_found());
        }
        let mtime = FileTime::from_last_modification_time(&data);
        let ts = time::Timespec {
            sec: mtime.unix_seconds() as i64,
            nsec: mtime.nanoseconds() as i32,
        };
        let tm = time::at(ts).to_utc();

        Response::builder()
            .header(header::CONTENT_TYPE, mime)
            .header(header::CONTENT_LENGTH, data.len())
            .header(
                header::LAST_MODIFIED,
                tm.strftime("%a, %d %b %Y %T GMT").unwrap().to_string(),
            )
            .body(Box::new(file) as Body)
            .map_err(box_error)
    }
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_LENGTH, 0)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Box::new(io::empty()) as Body)
        .unwrap()
}

#[cfg(test)]
mod tests {
    extern crate conduit_test as test;

    use std::fs::{self, File};
    use std::io::prelude::*;
    use tempdir::TempDir;

    use conduit::{header, Handler, Method, StatusCode};
    use Static;

    #[test]
    fn test_static() {
        let td = TempDir::new("conduit-static").unwrap();
        let root = td.path();
        let handler = Static::new(root.clone());
        File::create(&root.join("Cargo.toml"))
            .unwrap()
            .write_all(b"[package]")
            .unwrap();
        let mut req = test::MockRequest::new(Method::GET, "/Cargo.toml");
        let mut res = handler.call(&mut req).ok().expect("No response");
        let mut body = Vec::new();
        res.body_mut().write_body(&mut body).unwrap();
        assert_eq!(body, b"[package]");
        assert_eq!(
            res.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/plain"
        );
        assert_eq!(res.headers().get(header::CONTENT_LENGTH).unwrap(), "9");
    }

    #[test]
    fn test_mime_types() {
        let td = TempDir::new("conduit-static").unwrap();
        let root = td.path();
        fs::create_dir(&root.join("src")).unwrap();
        File::create(&root.join("src/fixture.css")).unwrap();

        let handler = Static::new(root.clone());
        let mut req = test::MockRequest::new(Method::GET, "/src/fixture.css");
        let res = handler.call(&mut req).ok().expect("No response");
        assert_eq!(res.headers().get(header::CONTENT_TYPE).unwrap(), "text/css");
        assert_eq!(res.headers().get(header::CONTENT_LENGTH).unwrap(), "0");
    }

    #[test]
    fn test_missing() {
        let td = TempDir::new("conduit-static").unwrap();
        let root = td.path();

        let handler = Static::new(root.clone());
        let mut req = test::MockRequest::new(Method::GET, "/nope");
        let res = handler.call(&mut req).ok().expect("No response");
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_dir() {
        let td = TempDir::new("conduit-static").unwrap();
        let root = td.path();

        fs::create_dir(&root.join("foo")).unwrap();

        let handler = Static::new(root.clone());
        let mut req = test::MockRequest::new(Method::GET, "/foo");
        let res = handler.call(&mut req).ok().expect("No response");
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn last_modified() {
        let td = TempDir::new("conduit-static").unwrap();
        let root = td.path();
        File::create(&root.join("test")).unwrap();
        let handler = Static::new(root.clone());
        let mut req = test::MockRequest::new(Method::GET, "/test");
        let res = handler.call(&mut req).ok().expect("No response");
        assert_eq!(res.status(), StatusCode::OK);
        assert!(res.headers().get(header::LAST_MODIFIED).is_some());
    }
}
