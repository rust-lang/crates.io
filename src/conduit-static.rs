extern crate mime = "mime-types";
extern crate conduit;

use std::fmt::Show;
use std::collections::HashMap;
use std::io::fs::File;
use std::io::util::NullReader;
use conduit::{Request, Response, Handler};

pub struct Static {
    path: Path,
    types: mime::Types
}

impl Static {
    pub fn new(path: Path) -> Static {
        Static {
            path: path,
            types: mime::Types::new()
                .ok().expect("Couldn't load mime-types")
        }
    }
}

impl Handler for Static {
    fn call(&self, request: &mut Request) -> Result<Response, Box<Show>> {
        let request_path = request.path().slice_from(1);
        let path = self.path.join(request_path);

        if !self.path.is_ancestor_of(&path) {
            return Ok(Response {
                status: (404, "Not Found"),
                headers: HashMap::new(),
                body: box NullReader
            })
        }

        let mime = self.types.mime_for_path(&path);
        let file = try!(File::open(&path).map_err(|e| box e as Box<Show>));

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_str(), vec!(mime.to_str()));

        Ok(Response {
            status: (200, "OK"),
            headers: headers,
            body: box file as Box<Reader + Send>
        })
    }
}

#[cfg(test)]
mod tests {
    extern crate test = "conduit-test";

    use std::io::{fs, File, TempDir, UserRWX};

    use conduit;
    use conduit::Handler;
    use Static;

    #[test]
    fn test_static() {
        let td = TempDir::new("conduit-static").unwrap();
        let root = td.path();
        let handler = Static::new(root.clone());
        File::create(&root.join("Cargo.toml")).write(b"[package]").unwrap();
        let mut req = test::MockRequest::new(conduit::Get, "/Cargo.toml");
        let mut res = handler.call(&mut req).ok().expect("No response");
        let body = res.body.read_to_str().ok().expect("No body");
        assert_eq!(body.as_slice(), "[package]");
        assert_eq!(res.headers.find_equiv(&"Content-Type")
                      .expect("No content-type"),
                   &vec!("text/plain".to_str()));
    }

    #[test]
    fn test_mime_types() {
        let td = TempDir::new("conduit-static").unwrap();
        let root = td.path();
        fs::mkdir(&root.join("src"), UserRWX).unwrap();
        File::create(&root.join("src/fixture.css")).unwrap();

        let handler = Static::new(root.clone());
        let mut req = test::MockRequest::new(conduit::Get, "/src/fixture.css");
        let res = handler.call(&mut req).ok().expect("No response");
        assert_eq!(res.headers.find_equiv(&"Content-Type")
                      .expect("No content-type"),
                   &vec!("text/css".to_str()));
    }
}
