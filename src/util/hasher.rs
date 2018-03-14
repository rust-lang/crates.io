use std::io::prelude::*;
use std::io;
use openssl::hash::{Hasher, MessageDigest};

pub fn hash(data: &[u8]) -> Vec<u8> {
    let mut hasher = Hasher::new(MessageDigest::sha256()).unwrap();
    hasher.update(data).unwrap();
    hasher.finish().unwrap().to_vec()
}

// Can't derive debug because of Hasher.
#[allow(missing_debug_implementations)]
pub struct HashingReader<R> {
    inner: R,
    hasher: Hasher,
}

impl<R: Read> HashingReader<R> {
    pub fn new(r: R) -> HashingReader<R> {
        HashingReader {
            inner: r,
            hasher: Hasher::new(MessageDigest::sha256()).unwrap(),
        }
    }

    pub fn finalize(mut self) -> Vec<u8> {
        self.hasher.finish().unwrap().to_vec()
    }
}

impl<R: Read> Read for HashingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let amt = self.inner.read(buf)?;
        self.hasher.update(&buf[..amt]).unwrap();
        Ok(amt)
    }
}
