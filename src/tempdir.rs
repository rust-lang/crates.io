use std::old_io;
use std::io;
use std::path;

pub struct TempDir {
    inner: old_io::TempDir,
}

impl TempDir {
    pub fn new(prefix: &str) -> io::Result<TempDir> {
        Ok(TempDir { inner: old_io::TempDir::new(prefix).unwrap() })
    }

    pub fn path(&self) -> &path::Path {
        path::Path::new(self.inner.path().as_str().unwrap())
    }
}

