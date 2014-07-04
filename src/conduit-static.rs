extern crate conduit;

use std::fmt::Show;
use std::collections::HashMap;
use std::io::fs::File;
use std::io::util::NullReader;
use conduit::{Request, Response, Handler};

pub struct Static {
    path: Path
}

impl Static {
    pub fn new(path: Path) -> Static {
        Static { path: path }
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

        let file = try!(File::open(&path).map_err(|e| box e as Box<Show>));

        Ok(Response {
            status: (200, "OK"),
            headers: HashMap::new(),
            body: box file as Box<Reader + Send>
        })
    }
}

#[cfg(test)]
mod tests {
    extern crate test = "conduit-test";
    use conduit;
    use conduit::Handler;
    use Static;

    #[test]
    fn test_static() {
        let root = Path::new(file!()).dir_path().dir_path();
        let handler = Static::new(root);
        let mut req = test::MockRequest::new(conduit::Get, "/Cargo.toml");
        let mut res = handler.call(&mut req).ok().expect("No response");
        let body = res.body.read_to_str().ok().expect("No body");
        assert!(body.as_slice().contains("[package]"), "The Cargo.toml was provided");
    }
}
